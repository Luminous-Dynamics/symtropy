// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Evidence-bound construction staging reservations.
//!
//! Construction staging reserves exact fabrication workpieces for exact site
//! plan steps. It does not own inventory, mass, transport, or physical location.
//! Matter bindings and fabrication lifecycle are captured only as revisioned
//! external snapshots; placement is proven by a separate external authority.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_fabrication::{
    FabricationPlan, FabricationPlanId, MatterBinding, PlanStepId, Workpiece, WorkpieceId,
    WorkpieceLifecycle,
};
use symtropy_game_state::StableId;

use crate::{ConstructionSite, ConstructionSiteId};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StagingReservationId(StableId);

impl StagingReservationId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for StagingReservationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// External attestation that one subject is at one semantic placement/location.
/// `subject_revision` is owned by the supplying authority; construction never
/// increments or interprets it beyond stale/equal/newer comparison.
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
        if digest.is_empty() || digest.len() > 256 {
            return Err(StagingError::InvalidPlacementDigest(digest));
        }
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

/// Immutable reference snapshot used only to detect whether a reserved
/// workpiece's fabrication/matter identity changed after reservation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedWorkpieceSnapshot {
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
/// It does not mean the workpiece was consumed, moved, or transformed.
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

/// Ephemeral validation result for a caller preparing to release work. This is
/// not persisted as an inventory flag or a guarantee that placement remains
/// current after the supplied evidence snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagingConfirmation {
    pub reservation_id: StagingReservationId,
    pub site_id: ConstructionSiteId,
    pub step_id: PlanStepId,
    pub workpiece_id: WorkpieceId,
    pub placement_evidence: PlacementEvidenceRef,
}

/// Construction-level reservation registry. A workpiece can have only one held
/// reservation in this ledger, preventing accidental construction double-booking
/// without claiming to be a global inventory authority outside this domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConstructionStagingLedger {
    reservations: Vec<StagingReservation>,
}

impl ConstructionStagingLedger {
    pub fn reservations(&self) -> &[StagingReservation] {
        &self.reservations
    }

    pub fn reservation(&self, id: &StagingReservationId) -> Option<&StagingReservation> {
        self.reservations.iter().find(|reservation| &reservation.id == id)
    }

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

    /// Validates that a held reservation still matches the exact fabrication
    /// snapshot and a newly supplied placement attestation for the same subject
    /// and semantic staging location.
    pub fn confirm_for_step(
        &self,
        reservation_id: &StagingReservationId,
        site: &ConstructionSite,
        plan: &FabricationPlan,
        step_id: &PlanStepId,
        workpiece: &Workpiece,
        placement: PlacementEvidenceRef,
    ) -> Result<StagingConfirmation, StagingError> {
        validate_site_plan(site, plan)?;
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
        if !reservation.snapshot.matches(workpiece) {
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

        Ok(StagingConfirmation {
            reservation_id: reservation_id.clone(),
            site_id: site.id.clone(),
            step_id: step_id.clone(),
            workpiece_id: workpiece.id.clone(),
            placement_evidence: placement,
        })
    }

    /// Releases only the construction reservation. It makes no claim that the
    /// workpiece moved, was consumed, or ceased to exist.
    pub fn release(
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
    if placement.digest.is_empty() || placement.digest.len() > 256 {
        return Err(StagingError::InvalidPlacementDigest(
            placement.digest.clone(),
        ));
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
    DuplicateReservationId(StagingReservationId),
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
            Self::DuplicateReservationId(id) => {
                write!(formatter, "staging reservation {id} already exists")
            }
            Self::WorkpieceAlreadyReserved(id) => {
                write!(formatter, "workpiece {id} already has a held construction reservation")
            }
            Self::PlanStepRequired => write!(formatter, "staging reservation requires at least one plan step"),
            Self::DuplicatePlanStep(id) => write!(formatter, "staging reservation repeats plan step {id}"),
            Self::UnknownPlanStep(id) => write!(formatter, "staging reservation references unknown plan step {id}"),
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
        }
    }
}

impl Error for StagingError {}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_fabrication::{PlanStep, ProcessSpecId};

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

    fn available_workpiece(name: &str, revision: u64) -> Workpiece {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id(&format!("workpiece:{name}"))),
            vec![binding(name, revision)],
        )
        .unwrap();
        workpiece.transition(WorkpieceLifecycle::Available).unwrap();
        workpiece
    }

    fn plan(workpiece: &Workpiece) -> FabricationPlan {
        FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:patch-conduit")),
            4,
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
                PlanStep::new(
                    PlanStepId::new(id("step:clamp")),
                    ProcessSpecId::new(id("process-spec:clamp")),
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

    #[test]
    fn reservation_is_exact_site_plan_workpiece_and_step_context() {
        let workpiece = available_workpiece("patch", 1);
        let plan = plan(&workpiece);
        let site = site(&plan);
        let mut ledger = ConstructionStagingLedger::default();
        let reservation_id = StagingReservationId::new(id("staging:patch"));

        ledger
            .reserve(
                reservation_id.clone(),
                &site,
                &plan,
                &workpiece,
                vec![
                    PlanStepId::new(id("step:align")),
                    PlanStepId::new(id("step:clamp")),
                ],
                placement(&workpiece, 7, "location:site-bench"),
            )
            .unwrap();

        let reservation = ledger.reservation(&reservation_id).unwrap();
        assert_eq!(reservation.snapshot.workpiece_id, workpiece.id);
        assert_eq!(reservation.step_ids().len(), 2);
        assert_eq!(reservation.lifecycle, StagingReservationLifecycle::Held);
    }

    #[test]
    fn same_workpiece_cannot_be_double_booked_while_held() {
        let workpiece = available_workpiece("patch", 1);
        let plan = plan(&workpiece);
        let site = site(&plan);
        let mut ledger = ConstructionStagingLedger::default();
        ledger
            .reserve(
                StagingReservationId::new(id("staging:first")),
                &site,
                &plan,
                &workpiece,
                vec![PlanStepId::new(id("step:align"))],
                placement(&workpiece, 7, "location:site-bench"),
            )
            .unwrap();

        let second = ledger.reserve(
            StagingReservationId::new(id("staging:second")),
            &site,
            &plan,
            &workpiece,
            vec![PlanStepId::new(id("step:clamp"))],
            placement(&workpiece, 8, "location:site-bench"),
        );
        assert!(matches!(
            second,
            Err(StagingError::WorkpieceAlreadyReserved(_))
        ));
    }

    #[test]
    fn changed_matter_snapshot_invalidates_staging_confirmation() {
        let workpiece = available_workpiece("patch", 1);
        let plan = plan(&workpiece);
        let site = site(&plan);
        let mut ledger = ConstructionStagingLedger::default();
        let reservation_id = StagingReservationId::new(id("staging:patch"));
        ledger
            .reserve(
                reservation_id.clone(),
                &site,
                &plan,
                &workpiece,
                vec![PlanStepId::new(id("step:align"))],
                placement(&workpiece, 7, "location:site-bench"),
            )
            .unwrap();

        let changed = available_workpiece("patch", 2);
        let result = ledger.confirm_for_step(
            &reservation_id,
            &site,
            &plan,
            &PlanStepId::new(id("step:align")),
            &changed,
            placement(&changed, 8, "location:site-bench"),
        );
        assert!(matches!(
            result,
            Err(StagingError::WorkpieceSnapshotChanged(_))
        ));
    }

    #[test]
    fn newer_matching_placement_evidence_confirms_without_becoming_location_authority() {
        let workpiece = available_workpiece("patch", 1);
        let plan = plan(&workpiece);
        let site = site(&plan);
        let mut ledger = ConstructionStagingLedger::default();
        let reservation_id = StagingReservationId::new(id("staging:patch"));
        ledger
            .reserve(
                reservation_id.clone(),
                &site,
                &plan,
                &workpiece,
                vec![PlanStepId::new(id("step:align"))],
                placement(&workpiece, 7, "location:site-bench"),
            )
            .unwrap();

        let confirmation = ledger
            .confirm_for_step(
                &reservation_id,
                &site,
                &plan,
                &PlanStepId::new(id("step:align")),
                &workpiece,
                placement(&workpiece, 9, "location:site-bench"),
            )
            .unwrap();
        assert_eq!(confirmation.workpiece_id, workpiece.id);
        assert_eq!(confirmation.placement_evidence.subject_revision, 9);
    }

    #[test]
    fn moved_or_stale_placement_evidence_is_rejected() {
        let workpiece = available_workpiece("patch", 1);
        let plan = plan(&workpiece);
        let site = site(&plan);
        let mut ledger = ConstructionStagingLedger::default();
        let reservation_id = StagingReservationId::new(id("staging:patch"));
        ledger
            .reserve(
                reservation_id.clone(),
                &site,
                &plan,
                &workpiece,
                vec![PlanStepId::new(id("step:align"))],
                placement(&workpiece, 7, "location:site-bench"),
            )
            .unwrap();

        let moved = ledger.confirm_for_step(
            &reservation_id,
            &site,
            &plan,
            &PlanStepId::new(id("step:align")),
            &workpiece,
            placement(&workpiece, 8, "location:warehouse"),
        );
        assert!(matches!(
            moved,
            Err(StagingError::PlacementContextChanged { .. })
        ));

        let stale = ledger.confirm_for_step(
            &reservation_id,
            &site,
            &plan,
            &PlanStepId::new(id("step:align")),
            &workpiece,
            placement(&workpiece, 6, "location:site-bench"),
        );
        assert!(matches!(
            stale,
            Err(StagingError::StalePlacementEvidence { .. })
        ));
    }

    #[test]
    fn release_frees_construction_reservation_without_consumption_claim() {
        let workpiece = available_workpiece("patch", 1);
        let plan = plan(&workpiece);
        let site = site(&plan);
        let mut ledger = ConstructionStagingLedger::default();
        let first = StagingReservationId::new(id("staging:first"));
        ledger
            .reserve(
                first.clone(),
                &site,
                &plan,
                &workpiece,
                vec![PlanStepId::new(id("step:align"))],
                placement(&workpiece, 7, "location:site-bench"),
            )
            .unwrap();
        ledger.release(&first, id("reason:restage")).unwrap();

        ledger
            .reserve(
                StagingReservationId::new(id("staging:second")),
                &site,
                &plan,
                &workpiece,
                vec![PlanStepId::new(id("step:clamp"))],
                placement(&workpiece, 8, "location:site-bench"),
            )
            .unwrap();

        let value = serde_json::to_value(ledger).unwrap();
        for forbidden in ["inventory", "quantity", "mass", "consumed", "transported", "progress"] {
            assert!(value.get(forbidden).is_none(), "unexpected field {forbidden}");
        }
    }
}

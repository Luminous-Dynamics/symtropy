// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact-authority staged execution proof for the Patch Conduit emergency bypass.
//!
//! This is the migration target for the live headless execution proof:
//!
//! exact F10 executable plan
//! -> `ExactConstructionSite` owns the immutable plan contract
//! -> C3 exact matter/placement staging
//! -> C2 release-time F5 capability admission
//! -> F4 process execution
//! -> `ValidatedProcessEvidence`
//! -> exact C4 release/process before-image continuity
//! -> exact C1 process-snapshot equality and completion
//! -> restage
//! -> pressure test
//! -> plan closure (not technical/civic commissioning).
//!
//! No public execution call in this module accepts a replacement
//! `FabricationPlan` or raw `ProcessEvidence`.

use crate::{
    patch_conduit::PatchConduitScenario,
    patch_conduit_exact_plan::{
        ExactPatchConduitApproach, ExactPatchConduitError, ExactPatchConduitProfile,
    },
    patch_conduit_execution::{PatchConduitExecutionProfile, PatchConduitProcessCatalog},
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_construction::{
    ConstructionSiteId, ConstructionSiteLifecycle, ConstructionStagingLedger,
    ExactConstructionSite, ExactContinuityError, ExactSiteError, PlacementEvidenceRef,
    SiteStepAdmissionId, StagingError, StagingReleaseInput, StagingReleaseRecordId,
    StagingReservationId, WorkActorRef, WorkOrderError, WorkOrderId, WorkOrderStatus,
    exact_work_order_status, issue_exact_work_order, record_exact_released_process_completion,
    release_exact_work_order, reserve_exact_workpiece,
};
use symtropy_fabrication::{
    CapabilityAdmission, CapabilityAdmissionId, CapabilityAxisRange, CapabilityEnvelope,
    CapabilityError, CapabilityEvidenceRef, CapabilityNeed, FabricationError, MatterBinding,
    PlanStepId, ProcessError, ProcessEvidenceValidationError, ProcessExecution, ProcessExecutionId,
    ProcessKind, ValidatedProcessEvidence, Workpiece, WorkpieceLifecycle,
};
use symtropy_game_state::StableId;

const BYPASS_SUBJECT: &str = "assembly:patch-conduit:emergency-bypass";
const INSTALL_STEP: &str = "step:firstlight:patch-conduit:serviceable-bypass-install";
const PRESSURE_STEP: &str = "step:firstlight:patch-conduit:serviceable-bypass-pressure-test";
const STAGING_LOCATION: &str = "location:firstlight:waterworks:repair-bench";
const PLACEMENT_AUTHORITY: &str = "authority:firstlight:waterworks-logistics";
const MATTER_AUTHORITY: &str = "matter:firstlight:reference-physical";

const WORKPIECE_NAMES: [&str; 4] = [
    "ruptured-line",
    "serviceable-bypass-hose",
    "serviceable-adapter-upstream",
    "serviceable-adapter-downstream",
];

/// Evidence summary for the exact-authority reference execution.
/// `PlanClosed` remains orchestration truth only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactReferenceBypassExecutionReport {
    pub plan_id: String,
    pub plan_revision: u64,
    pub completed_steps: usize,
    pub exact_completion_records: usize,
    pub staging_release_records: usize,
    pub final_matter_revision: u64,
    pub site_lifecycle: ConstructionSiteLifecycle,
    pub missing_pressure_measurement_rejected: bool,
    pub bypass_removal_planned: bool,
    pub validated_wire_evidence: bool,
    pub caller_supplied_plan_execution_boundary: bool,
}

/// Runs the smallest complete construction/fabrication path through the exact
/// authority boundary.
pub fn run_exact_reference_bypass_execution()
-> Result<ExactReferenceBypassExecutionReport, ExactReferenceBypassExecutionError> {
    let scenario = PatchConduitScenario::canonical()
        .map_err(|error| ExactReferenceBypassExecutionError::Scenario(error.to_string()))?;
    let compiled = PatchConduitExecutionProfile::compile(&scenario)
        .map_err(|error| ExactReferenceBypassExecutionError::ExecutionProfile(error.to_string()))?;
    let exact = ExactPatchConduitProfile::from_compiled(&compiled)?;
    let approach = exact
        .approach_for_subject(&assembly_subject(BYPASS_SUBJECT))
        .ok_or_else(|| invariant("reference bypass approach is missing from exact profile"))?;
    execute_exact_reference_bypass(&compiled.catalog, approach)
}

fn execute_exact_reference_bypass(
    catalog: &PatchConduitProcessCatalog,
    approach: &ExactPatchConduitApproach,
) -> Result<ExactReferenceBypassExecutionReport, ExactReferenceBypassExecutionError> {
    let install_step = plan_step(INSTALL_STEP);
    let pressure_step = plan_step(PRESSURE_STEP);
    let install_contract = catalog
        .process_for_step(&install_step)
        .ok_or_else(|| invariant("bypass install has no process contract"))?;
    let pressure_contract = catalog
        .process_for_step(&pressure_step)
        .ok_or_else(|| invariant("bypass pressure test has no process contract"))?;

    if install_contract.spec.kind != ProcessKind::Couple {
        return Err(invariant("bypass install is not bound to Couple"));
    }
    if pressure_contract.spec.kind != ProcessKind::PressureTest {
        return Err(invariant(
            "bypass verification is not bound to PressureTest",
        ));
    }

    let install_snapshot = install_contract.spec.snapshot();
    let pressure_snapshot = pressure_contract.spec.snapshot();
    if approach.plan.process_spec(&install_step) != Some(&install_snapshot) {
        return Err(invariant(
            "exact bypass plan does not bind the catalog Couple semantics",
        ));
    }
    if approach.plan.process_spec(&pressure_step) != Some(&pressure_snapshot) {
        return Err(invariant(
            "exact bypass plan does not bind the catalog PressureTest semantics",
        ));
    }

    let mut site = ExactConstructionSite::new(
        ConstructionSiteId::new(sid("construction-site:firstlight:patch-conduit:bypass")),
        approach.plan.clone(),
    );
    let mut staging = ConstructionStagingLedger::default();

    // Phase 1: stage the exact pre-coupling matter snapshot.
    let before = reference_workpieces(1)?;
    let install_reservations =
        reserve_for_step(&mut staging, &site, &install_step, &before, "couple", 1)?;

    let coupling_admission = admission_for_capability(
        catalog,
        install_contract.capability_need_ids(),
        "capability:fluid-coupling-control",
        reference_coupling_envelope()?,
        "couple",
    )?;
    let install_execution_id = ProcessExecutionId::new(sid(
        "process-execution:firstlight:patch-conduit:bypass-couple",
    ));
    let mut install_order = issue_exact_work_order(
        WorkOrderId::new(sid("work-order:firstlight:patch-conduit:bypass-couple")),
        &site,
        install_step.clone(),
        reference_actor(),
    )?;
    release_exact_work_order(
        &mut staging,
        StagingReleaseRecordId::new(sid(
            "staging-release:firstlight:patch-conduit:bypass-couple",
        )),
        &mut install_order,
        &mut site,
        SiteStepAdmissionId::new(sid("site-admission:firstlight:patch-conduit:bypass-couple")),
        install_execution_id.clone(),
        std::slice::from_ref(&coupling_admission),
        release_inputs(&install_reservations, &before, "couple", 2)?,
    )?;

    let bootstrap = install_order.bootstrap_evidence().ok_or_else(|| {
        invariant("successful exact Couple release emitted no F4 bootstrap evidence")
    })?;
    let before_refs = workpiece_refs(&before);
    let mut install_execution = ProcessExecution::begin(
        install_execution_id,
        &install_contract.spec,
        &before_refs,
        &bootstrap,
    )?;

    let after_couple = reference_workpieces(2)?;
    let install_evidence = install_execution.complete(
        sid(MATTER_AUTHORITY),
        sid("process-evidence:firstlight:patch-conduit:bypass-couple"),
        2,
        "digest:process:firstlight:patch-conduit:bypass-couple:2",
        resulting_bindings(&after_couple),
    )?;
    let install_evidence = ValidatedProcessEvidence::new(install_evidence)?;
    let completed =
        record_exact_released_process_completion(&staging, &mut site, &install_evidence)?;
    if completed != install_step {
        return Err(invariant(
            "exact C4/C1 completed the wrong bypass install step",
        ));
    }
    if exact_work_order_status(&install_order, &site)? != WorkOrderStatus::Completed {
        return Err(invariant(
            "bypass install work order did not derive Completed from exact C1",
        ));
    }

    // The coupling changed physical matter. Release the old construction claims
    // and restage the exact post-process before-image for testing.
    for reservation in &install_reservations {
        staging.release_reservation(
            reservation,
            sid("reason:firstlight:patch-conduit:post-couple-restage"),
        )?;
    }
    let pressure_reservations = reserve_for_step(
        &mut staging,
        &site,
        &pressure_step,
        &after_couple,
        "pressure-test",
        3,
    )?;

    let source_admission = admission_for_capability(
        catalog,
        pressure_contract.capability_need_ids(),
        "capability:hydrostatic-pressure-source",
        reference_pressure_source_envelope()?,
        "pressure-source",
    )?;
    let measurement_admission = admission_for_capability(
        catalog,
        pressure_contract.capability_need_ids(),
        "capability:pressure-measurement",
        reference_pressure_measurement_envelope()?,
        "pressure-measurement",
    )?;

    let mut pressure_order = issue_exact_work_order(
        WorkOrderId::new(sid(
            "work-order:firstlight:patch-conduit:bypass-pressure-test",
        )),
        &site,
        pressure_step.clone(),
        reference_actor(),
    )?;

    // Negative proof: a pressure source is not a pressure measurement system.
    // Missing the calibrated measurement admission must fail without mutating
    // exact site or release state.
    let site_revision_before_rejection = site.revision();
    let admissions_before_rejection = site.admissions().len();
    let releases_before_rejection = staging.releases().len();
    let rejected = release_exact_work_order(
        &mut staging,
        StagingReleaseRecordId::new(sid(
            "staging-release:firstlight:patch-conduit:pressure-missing-gauge",
        )),
        &mut pressure_order,
        &mut site,
        SiteStepAdmissionId::new(sid(
            "site-admission:firstlight:patch-conduit:pressure-missing-gauge",
        )),
        ProcessExecutionId::new(sid(
            "process-execution:firstlight:patch-conduit:pressure-missing-gauge",
        )),
        std::slice::from_ref(&source_admission),
        release_inputs(&pressure_reservations, &after_couple, "pressure-test", 4)?,
    );
    let missing_pressure_measurement_rejected = matches!(
        rejected,
        Err(StagingError::WorkOrder(
            WorkOrderError::MissingCapabilityCoverage(_)
        ))
    );
    if !missing_pressure_measurement_rejected {
        return Err(invariant(
            "exact pressure-test release did not fail on missing capability coverage",
        ));
    }
    if site.revision() != site_revision_before_rejection
        || site.admissions().len() != admissions_before_rejection
        || staging.releases().len() != releases_before_rejection
        || pressure_order.bootstrap_evidence().is_some()
    {
        return Err(invariant(
            "failed exact pressure-test capability gate mutated release/site state",
        ));
    }

    let pressure_execution_id = ProcessExecutionId::new(sid(
        "process-execution:firstlight:patch-conduit:bypass-pressure-test",
    ));
    release_exact_work_order(
        &mut staging,
        StagingReleaseRecordId::new(sid(
            "staging-release:firstlight:patch-conduit:bypass-pressure-test",
        )),
        &mut pressure_order,
        &mut site,
        SiteStepAdmissionId::new(sid(
            "site-admission:firstlight:patch-conduit:bypass-pressure-test",
        )),
        pressure_execution_id.clone(),
        &[source_admission, measurement_admission],
        release_inputs(&pressure_reservations, &after_couple, "pressure-test", 4)?,
    )?;

    let pressure_bootstrap = pressure_order.bootstrap_evidence().ok_or_else(|| {
        invariant("successful exact PressureTest release emitted no F4 bootstrap evidence")
    })?;
    if pressure_bootstrap.len() != 2 {
        return Err(invariant(
            "exact pressure test did not preserve both F5 admissions into F4",
        ));
    }

    let after_refs = workpiece_refs(&after_couple);
    let mut pressure_execution = ProcessExecution::begin(
        pressure_execution_id,
        &pressure_contract.spec,
        &after_refs,
        &pressure_bootstrap,
    )?;
    let pressure_evidence = pressure_execution.complete(
        sid(MATTER_AUTHORITY),
        sid("process-evidence:firstlight:patch-conduit:bypass-pressure-test"),
        3,
        "digest:process:firstlight:patch-conduit:bypass-pressure-test:3",
        resulting_bindings(&after_couple),
    )?;
    let pressure_evidence = ValidatedProcessEvidence::new(pressure_evidence)?;
    let completed =
        record_exact_released_process_completion(&staging, &mut site, &pressure_evidence)?;
    if completed != pressure_step {
        return Err(invariant(
            "exact C4/C1 completed the wrong pressure-test step",
        ));
    }
    if exact_work_order_status(&pressure_order, &site)? != WorkOrderStatus::Completed {
        return Err(invariant(
            "pressure-test work order did not derive Completed from exact C1",
        ));
    }

    if !site.plan_complete()? {
        return Err(invariant(
            "two-step exact bypass plan did not become complete after two evidence-backed steps",
        ));
    }
    site.close_plan()?;
    if site.lifecycle() != ConstructionSiteLifecycle::PlanClosed {
        return Err(invariant(
            "completed exact bypass plan did not enter PlanClosed",
        ));
    }

    if site.exact_completions().len() != 2 {
        return Err(invariant(
            "exact site did not retain one exact semantic completion per step",
        ));
    }

    let bypass_removal_planned = approach
        .temporary_works
        .iter()
        .any(|work| work.removal_step_id.is_some());
    Ok(ExactReferenceBypassExecutionReport {
        plan_id: site.plan_id().stable_id().as_str().to_owned(),
        plan_revision: site.plan_revision(),
        completed_steps: site.completed_step_ids().len(),
        exact_completion_records: site.exact_completions().len(),
        staging_release_records: staging.releases().len(),
        final_matter_revision: 2,
        site_lifecycle: site.lifecycle(),
        missing_pressure_measurement_rejected,
        bypass_removal_planned,
        validated_wire_evidence: true,
        caller_supplied_plan_execution_boundary: false,
    })
}

fn reference_actor() -> WorkActorRef {
    WorkActorRef {
        authority_id: sid("authority:firstlight:residents"),
        actor_id: sid("resident:firstlight:waterworks-technician"),
        actor_revision: 1,
    }
}

fn reference_workpieces(revision: u64) -> Result<Vec<Workpiece>, FabricationError> {
    WORKPIECE_NAMES
        .iter()
        .map(|name| {
            let mut workpiece = Workpiece::new(
                symtropy_fabrication::WorkpieceId::new(sid(format!(
                    "workpiece:firstlight:patch-conduit:{name}"
                ))),
                vec![MatterBinding::new(
                    sid(MATTER_AUTHORITY),
                    sid(format!("allocation:firstlight:patch-conduit:{name}")),
                    revision,
                    format!("digest:matter:firstlight:patch-conduit:{name}:{revision}"),
                )?],
            )?;
            workpiece.transition(WorkpieceLifecycle::Available)?;
            Ok(workpiece)
        })
        .collect()
}

fn resulting_bindings(workpieces: &[Workpiece]) -> Vec<MatterBinding> {
    workpieces
        .iter()
        .flat_map(|workpiece| workpiece.matter_bindings.iter().cloned())
        .collect()
}

fn workpiece_refs(workpieces: &[Workpiece]) -> Vec<&Workpiece> {
    workpieces.iter().collect()
}

fn reserve_for_step(
    staging: &mut ConstructionStagingLedger,
    site: &ExactConstructionSite,
    step_id: &PlanStepId,
    workpieces: &[Workpiece],
    phase: &str,
    placement_revision: u64,
) -> Result<Vec<StagingReservationId>, StagingError> {
    let mut reservations = Vec::with_capacity(workpieces.len());
    for (index, workpiece) in workpieces.iter().enumerate() {
        let reservation = StagingReservationId::new(sid(format!(
            "staging-reservation:firstlight:patch-conduit:{phase}:{index}"
        )));
        reserve_exact_workpiece(
            staging,
            reservation.clone(),
            site,
            workpiece,
            vec![step_id.clone()],
            placement(workpiece, phase, index, placement_revision)?,
        )?;
        reservations.push(reservation);
    }
    Ok(reservations)
}

fn release_inputs<'a>(
    reservations: &'a [StagingReservationId],
    workpieces: &'a [Workpiece],
    phase: &str,
    placement_revision: u64,
) -> Result<Vec<StagingReleaseInput<'a>>, StagingError> {
    if reservations.len() != workpieces.len() {
        return Err(StagingError::PlanStepRequired);
    }
    reservations
        .iter()
        .zip(workpieces)
        .enumerate()
        .map(|(index, (reservation_id, workpiece))| {
            Ok(StagingReleaseInput {
                reservation_id,
                workpiece,
                placement: placement(workpiece, phase, index, placement_revision)?,
            })
        })
        .collect()
}

fn placement(
    workpiece: &Workpiece,
    phase: &str,
    index: usize,
    subject_revision: u64,
) -> Result<PlacementEvidenceRef, StagingError> {
    PlacementEvidenceRef::new(
        sid(PLACEMENT_AUTHORITY),
        sid(format!(
            "placement-evidence:firstlight:patch-conduit:{phase}:{index}:{subject_revision}"
        )),
        workpiece.id.stable_id().clone(),
        subject_revision,
        sid(STAGING_LOCATION),
        format!("digest:placement:firstlight:patch-conduit:{phase}:{index}:{subject_revision}"),
    )
}

fn admission_for_capability(
    catalog: &PatchConduitProcessCatalog,
    allowed_need_ids: &[symtropy_fabrication::CapabilityNeedId],
    capability_id: &str,
    envelope: CapabilityEnvelope,
    suffix: &str,
) -> Result<CapabilityAdmission, ExactReferenceBypassExecutionError> {
    let need = find_need(catalog, allowed_need_ids, capability_id)?;
    let admission = need.evaluate(
        CapabilityAdmissionId::new(sid(format!(
            "capability-admission:firstlight:patch-conduit:{suffix}"
        ))),
        &envelope,
    );
    if !admission.is_satisfied() {
        return Err(invariant(
            "reference provider envelope failed its intended capability need",
        ));
    }
    Ok(admission)
}

fn find_need<'a>(
    catalog: &'a PatchConduitProcessCatalog,
    allowed_need_ids: &[symtropy_fabrication::CapabilityNeedId],
    capability_id: &str,
) -> Result<&'a CapabilityNeed, ExactReferenceBypassExecutionError> {
    allowed_need_ids
        .iter()
        .filter_map(|id| catalog.capability_need(id))
        .find(|need| need.capability_id == sid(capability_id))
        .ok_or_else(|| invariant("process contract lacks expected capability need"))
}

fn reference_coupling_envelope() -> Result<CapabilityEnvelope, CapabilityError> {
    CapabilityEnvelope::new(
        sid("provider:firstlight:field-coupling-tool"),
        3,
        sid("capability:fluid-coupling-control"),
        sid("mode:firstlight:field-coupling"),
        vec![CapabilityAxisRange::new(
            sid("axis:coupling-torque-cnm"),
            0,
            8_000,
            10,
        )?],
        vec![sid("condition:coupling-accessible")],
        capability_evidence("coupling-tool", 3)?,
    )
}

fn reference_pressure_source_envelope() -> Result<CapabilityEnvelope, CapabilityError> {
    CapabilityEnvelope::new(
        sid("provider:firstlight:hydrostatic-pump"),
        4,
        sid("capability:hydrostatic-pressure-source"),
        sid("mode:firstlight:hydrostatic-test"),
        vec![CapabilityAxisRange::new(
            sid("axis:pressure-kpa"),
            0,
            1_200,
            5,
        )?],
        vec![sid("condition:service-line-isolated")],
        capability_evidence("hydrostatic-pump", 4)?,
    )
}

fn reference_pressure_measurement_envelope() -> Result<CapabilityEnvelope, CapabilityError> {
    CapabilityEnvelope::new(
        sid("provider:firstlight:calibrated-pressure-gauge"),
        8,
        sid("capability:pressure-measurement"),
        sid("mode:firstlight:hydrostatic-test"),
        vec![
            CapabilityAxisRange::new(sid("axis:pressure-kpa"), 0, 1_200, 5)?,
            CapabilityAxisRange::new(sid("axis:hold-duration-s"), 0, 900, 1)?,
        ],
        vec![],
        capability_evidence("calibrated-pressure-gauge", 8)?,
    )
}

fn capability_evidence(
    provider: &str,
    revision: u64,
) -> Result<CapabilityEvidenceRef, CapabilityError> {
    CapabilityEvidenceRef::new(
        sid("authority:firstlight:tool-calibration"),
        sid(format!(
            "capability-evidence:firstlight:{provider}:{revision}"
        )),
        revision,
        format!("digest:capability:firstlight:{provider}:{revision}"),
    )
}

fn plan_step(value: &str) -> PlanStepId {
    PlanStepId::new(sid(value))
}

fn assembly_subject(value: &str) -> symtropy_fabrication::FunctionalSubject {
    symtropy_fabrication::FunctionalSubject {
        kind: symtropy_fabrication::FunctionalSubjectKind::Assembly,
        id: sid(value),
    }
}

fn sid(value: impl Into<String>) -> StableId {
    StableId::parse(value).expect("Patch Conduit exact staged-execution identifiers must be valid")
}

fn invariant(message: &'static str) -> ExactReferenceBypassExecutionError {
    ExactReferenceBypassExecutionError::Invariant(message)
}

#[derive(Debug)]
pub enum ExactReferenceBypassExecutionError {
    Scenario(String),
    ExecutionProfile(String),
    ExactPlan(ExactPatchConduitError),
    Invariant(&'static str),
    Fabrication(FabricationError),
    Capability(CapabilityError),
    Process(ProcessError),
    ProcessEvidenceValidation(ProcessEvidenceValidationError),
    WorkOrder(WorkOrderError),
    Staging(StagingError),
    ExactContinuity(ExactContinuityError),
    ExactSite(ExactSiteError),
}

impl From<ExactPatchConduitError> for ExactReferenceBypassExecutionError {
    fn from(error: ExactPatchConduitError) -> Self {
        Self::ExactPlan(error)
    }
}

impl From<FabricationError> for ExactReferenceBypassExecutionError {
    fn from(error: FabricationError) -> Self {
        Self::Fabrication(error)
    }
}

impl From<CapabilityError> for ExactReferenceBypassExecutionError {
    fn from(error: CapabilityError) -> Self {
        Self::Capability(error)
    }
}

impl From<ProcessError> for ExactReferenceBypassExecutionError {
    fn from(error: ProcessError) -> Self {
        Self::Process(error)
    }
}

impl From<ProcessEvidenceValidationError> for ExactReferenceBypassExecutionError {
    fn from(error: ProcessEvidenceValidationError) -> Self {
        Self::ProcessEvidenceValidation(error)
    }
}

impl From<WorkOrderError> for ExactReferenceBypassExecutionError {
    fn from(error: WorkOrderError) -> Self {
        Self::WorkOrder(error)
    }
}

impl From<StagingError> for ExactReferenceBypassExecutionError {
    fn from(error: StagingError) -> Self {
        Self::Staging(error)
    }
}

impl From<ExactContinuityError> for ExactReferenceBypassExecutionError {
    fn from(error: ExactContinuityError) -> Self {
        Self::ExactContinuity(error)
    }
}

impl From<ExactSiteError> for ExactReferenceBypassExecutionError {
    fn from(error: ExactSiteError) -> Self {
        Self::ExactSite(error)
    }
}

impl fmt::Display for ExactReferenceBypassExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scenario(error) => write!(formatter, "Patch Conduit scenario failed: {error}"),
            Self::ExecutionProfile(error) => {
                write!(formatter, "Patch Conduit execution profile failed: {error}")
            }
            Self::ExactPlan(error) => fmt::Display::fmt(error, formatter),
            Self::Invariant(message) => write!(
                formatter,
                "Patch Conduit exact staged-execution invariant failed: {message}"
            ),
            Self::Fabrication(error) => fmt::Display::fmt(error, formatter),
            Self::Capability(error) => fmt::Display::fmt(error, formatter),
            Self::Process(error) => fmt::Display::fmt(error, formatter),
            Self::ProcessEvidenceValidation(error) => fmt::Display::fmt(error, formatter),
            Self::WorkOrder(error) => fmt::Display::fmt(error, formatter),
            Self::Staging(error) => fmt::Display::fmt(error, formatter),
            Self::ExactContinuity(error) => fmt::Display::fmt(error, formatter),
            Self::ExactSite(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl Error for ExactReferenceBypassExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ExactPlan(error) => Some(error),
            Self::Fabrication(error) => Some(error),
            Self::Capability(error) => Some(error),
            Self::Process(error) => Some(error),
            Self::ProcessEvidenceValidation(error) => Some(error),
            Self::WorkOrder(error) => Some(error),
            Self::Staging(error) => Some(error),
            Self::ExactContinuity(error) => Some(error),
            Self::ExactSite(error) => Some(error),
            Self::Scenario(_) | Self::ExecutionProfile(_) | Self::Invariant(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emergency_bypass_executes_through_exact_plan_staging_capability_continuity_and_site() {
        let report = run_exact_reference_bypass_execution().unwrap();
        assert_eq!(report.plan_revision, 2);
        assert_eq!(report.completed_steps, 2);
        assert_eq!(report.exact_completion_records, 2);
        assert_eq!(report.staging_release_records, 2);
        assert_eq!(report.final_matter_revision, 2);
        assert_eq!(report.site_lifecycle, ConstructionSiteLifecycle::PlanClosed);
        assert!(report.missing_pressure_measurement_rejected);
        assert!(report.validated_wire_evidence);
        assert!(!report.caller_supplied_plan_execution_boundary);
        assert!(!report.bypass_removal_planned);
    }

    #[test]
    fn exact_plan_closed_report_does_not_claim_commissioning_or_authorization() {
        let report = run_exact_reference_bypass_execution().unwrap();
        let json = serde_json::to_string(&report).unwrap();
        for forbidden in [
            "commissioned",
            "authorized",
            "safe",
            "restored_service",
            "device_bus",
            "quality",
            "score",
        ] {
            assert!(!json.contains(forbidden), "unexpected claim {forbidden}");
        }
    }
}

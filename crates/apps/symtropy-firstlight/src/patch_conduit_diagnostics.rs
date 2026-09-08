// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pressure-test evidence -> engineering facts -> diagnostic reasoning for Patch Conduit.
//!
//! This bridge keeps three claims separate:
//! - F4 proves that an exact PressureTest process completed;
//! - F6 observations carry the instrument measurements made during that process;
//! - F7/F11 decide what those observations imply for the functional design and
//!   competing diagnostic hypotheses.
//!
//! A completed test is therefore never interpreted as a passed test.

use crate::{
    patch_conduit::PatchConduitScenario,
    patch_conduit_execution::{PatchConduitExecutionProfile, PatchConduitProcessContract},
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_fabrication::{
    CapabilityEvidence, ConstraintError, DesignBinding, DesignRoleId, DiagnosticAssessment,
    DiagnosticCase, DiagnosticCaseId, DiagnosticCriterion, DiagnosticError, DiagnosticHypothesis,
    DiagnosticHypothesisId, DiagnosticProbe, DiagnosticProbeId, EngineeringEvidenceRef,
    EngineeringFact, EngineeringFactId, EngineeringFactSet, EngineeringFactValue, FabricationError,
    FunctionalConstraintId, FunctionalSubject, FunctionalSubjectKind, HypothesisOutcome,
    KnownConstraintState, MatterBinding, MeasurementInterval, ObservationState, PlanStepId,
    ProcessError, ProcessEvidence, ProcessExecution, ProcessExecutionId, ProcessExecutionState,
    ProcessKind, RoleBinding, WorkmanshipError, WorkmanshipEvidenceRef, WorkmanshipObservation,
    WorkmanshipObservationId, WorkmanshipValue, WorkmanshipVector, Workpiece, WorkpieceId,
    WorkpieceLifecycle,
};
use symtropy_game_state::StableId;

const REPAIR_ROLE: &str = "role:patch-conduit:repair-assembly";
const BYPASS_SUBJECT: &str = "assembly:patch-conduit:emergency-bypass";

const CONSTRAINT_PRESSURE_LOSS: &str = "constraint:patch-conduit:pressure-loss";
const CONSTRAINT_CONTAINMENT: &str = "constraint:patch-conduit:containment";
const CONSTRAINT_WATER_COMPATIBILITY: &str = "constraint:patch-conduit:water-compatibility";
const CONSTRAINT_NO_ACTIVE_LEAK: &str = "constraint:patch-conduit:no-active-leak";

const DIM_PRESSURE_LOSS: &str = "engineering:pressure-loss-pa";
const DIM_CONTAINMENT: &str = "engineering:pressure-containment-kpa";
const DIM_WATER_COMPATIBILITY: &str = "engineering:water-service-compatibility";
const DIM_ACTIVE_LEAK: &str = "engineering:active-leak";

const OBS_PRESSURE_LOSS: &str = "observation:patch-conduit:pressure-loss-pa";
const OBS_CONTAINMENT: &str = "observation:patch-conduit:pressure-containment-kpa";
const OBS_ACTIVE_LEAK: &str = "observation:patch-conduit:active-leak";

const PRESSURE_STEP: &str = "step:firstlight:patch-conduit:serviceable-bypass-pressure-test";
const INSPECT_STEP: &str = "step:firstlight:patch-conduit:salvage-inspect";

const HYPOTHESIS_COUPLING_LEAK: &str = "hypothesis:patch-conduit:coupling-or-seal-leak";
const HYPOTHESIS_RESTRICTION: &str = "hypothesis:patch-conduit:hydraulic-restriction";
const HYPOTHESIS_MATERIAL: &str = "hypothesis:patch-conduit:material-incompatibility";
const PROBE_VISUAL_LEAK: &str = "probe:patch-conduit:visual-leak-inspection";
const PROBE_REPEAT_PRESSURE: &str = "probe:patch-conduit:repeat-pressure-test";

/// Compact headless proof result. No probability, score, selected diagnosis,
/// commissioning, or authorization field exists here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchConduitDiagnosticReport {
    pub pressure_process_completed: bool,
    pub known_failure_all_satisfied: bool,
    pub supported_hypotheses: Vec<String>,
    pub contradicted_hypotheses: Vec<String>,
    pub unresolved_without_leak_localization: Vec<String>,
    pub suggested_probes_without_leak_localization: Vec<String>,
}

/// Runs two assessments against one exact completed pressure-test execution:
/// 1. known leak evidence -> coupling/seal fault supported;
/// 2. leak localization omitted -> the same hypothesis remains unresolved and
///    F11 proposes an inspection probe.
pub fn run_reference_pressure_diagnostics()
-> Result<PatchConduitDiagnosticReport, PatchConduitDiagnosticError> {
    let scenario = PatchConduitScenario::canonical()
        .map_err(|error| PatchConduitDiagnosticError::Scenario(error.to_string()))?;
    let profile = PatchConduitExecutionProfile::compile(&scenario)
        .map_err(|error| PatchConduitDiagnosticError::ExecutionProfile(error.to_string()))?;
    let pressure_contract = profile
        .catalog
        .process_for_step(&plan_step(PRESSURE_STEP))
        .ok_or(PatchConduitDiagnosticError::MissingPressureContract)?;
    let inspect_contract = profile
        .catalog
        .process_for_step(&plan_step(INSPECT_STEP))
        .ok_or(PatchConduitDiagnosticError::MissingInspectContract)?;

    if pressure_contract.spec.kind != ProcessKind::PressureTest {
        return Err(PatchConduitDiagnosticError::WrongProcessKind {
            expected: ProcessKind::PressureTest,
            actual: pressure_contract.spec.kind,
        });
    }
    if inspect_contract.spec.kind != ProcessKind::Inspect {
        return Err(PatchConduitDiagnosticError::WrongProcessKind {
            expected: ProcessKind::Inspect,
            actual: inspect_contract.spec.kind,
        });
    }

    let pressure_evidence = reference_pressure_process_evidence(pressure_contract)?;
    let diagnostic_case = diagnostic_case(&scenario, pressure_contract, inspect_contract)?;
    let binding = bypass_binding()?;

    let known_vector =
        reference_pressure_observations(&pressure_evidence, Some(ObservationState::Present))?;
    let known_facts = engineering_facts_from_pressure_test(
        &pressure_evidence,
        &known_vector,
        material_compatibility_fact()?,
    )?;
    let known_evaluation = scenario.design.evaluate(&binding, &known_facts);
    let known_assessment = diagnostic_case.assess(&known_evaluation)?;

    let unresolved_vector = reference_pressure_observations(&pressure_evidence, None)?;
    let unresolved_facts = engineering_facts_from_pressure_test(
        &pressure_evidence,
        &unresolved_vector,
        material_compatibility_fact()?,
    )?;
    let unresolved_evaluation = scenario.design.evaluate(&binding, &unresolved_facts);
    let unresolved_assessment = diagnostic_case.assess(&unresolved_evaluation)?;

    Ok(PatchConduitDiagnosticReport {
        pressure_process_completed: pressure_evidence.outcome == ProcessExecutionState::Completed,
        known_failure_all_satisfied: known_evaluation.all_satisfied(),
        supported_hypotheses: hypothesis_ids(&known_assessment, HypothesisOutcome::Supported),
        contradicted_hypotheses: hypothesis_ids(&known_assessment, HypothesisOutcome::Contradicted),
        unresolved_without_leak_localization: hypothesis_ids(
            &unresolved_assessment,
            HypothesisOutcome::Unresolved,
        ),
        suggested_probes_without_leak_localization: unresolved_assessment
            .suggested_probes
            .iter()
            .map(|id| id.stable_id().as_str().to_owned())
            .collect(),
    })
}

/// Converts only explicitly mapped instrument observations into engineering
/// facts, after proving that the F6 vector belongs to the exact completed F4
/// PressureTest execution. Material compatibility is supplied separately because
/// a pressure test cannot establish material identity by itself.
pub fn engineering_facts_from_pressure_test(
    process: &ProcessEvidence,
    observations: &WorkmanshipVector,
    material_fact: EngineeringFact,
) -> Result<EngineeringFactSet, PatchConduitDiagnosticError> {
    validate_pressure_observation_context(process, observations)?;

    let subject = bypass_subject();
    let mut facts = Vec::new();
    for observation in observations.observations() {
        let Some((dimension, value)) = map_pressure_observation(observation)? else {
            continue;
        };
        facts.push(EngineeringFact {
            id: EngineeringFactId::new(sid(format!(
                "engineering-fact:firstlight:{}",
                observation.id.stable_id().as_str()
            ))),
            subject: subject.clone(),
            dimension_id: sid(dimension),
            value,
            evidence: EngineeringEvidenceRef::new(
                observation.evidence.authority_id.clone(),
                observation.evidence.evidence_id.clone(),
                observation.evidence.revision,
                observation.evidence.digest.clone(),
            )?,
        });
    }

    if facts.is_empty() {
        return Err(PatchConduitDiagnosticError::NoMappedPressureObservations);
    }
    if material_fact.subject != subject
        || material_fact.dimension_id != sid(DIM_WATER_COMPATIBILITY)
    {
        return Err(PatchConduitDiagnosticError::InvalidMaterialFact);
    }
    facts.push(material_fact);
    Ok(EngineeringFactSet::new(facts)?)
}

fn validate_pressure_observation_context(
    process: &ProcessEvidence,
    observations: &WorkmanshipVector,
) -> Result<(), PatchConduitDiagnosticError> {
    if process.outcome != ProcessExecutionState::Completed {
        return Err(PatchConduitDiagnosticError::ProcessNotCompleted);
    }
    if process.kind != ProcessKind::PressureTest {
        return Err(PatchConduitDiagnosticError::WrongProcessKind {
            expected: ProcessKind::PressureTest,
            actual: process.kind,
        });
    }
    if process.inputs.is_empty() || process.resulting_matter.is_empty() {
        return Err(PatchConduitDiagnosticError::MalformedProcessEvidence);
    }
    if observations.execution_id != process.execution_id
        || observations.spec_id != process.spec_id
        || observations.spec_revision != process.spec_revision
    {
        return Err(PatchConduitDiagnosticError::ObservationContextMismatch);
    }
    Ok(())
}

fn map_pressure_observation(
    observation: &WorkmanshipObservation,
) -> Result<Option<(&'static str, EngineeringFactValue)>, PatchConduitDiagnosticError> {
    match observation.dimension_id.as_str() {
        OBS_PRESSURE_LOSS => match &observation.value {
            WorkmanshipValue::Measurement { interval } => Ok(Some((
                DIM_PRESSURE_LOSS,
                EngineeringFactValue::Measurement {
                    interval: interval.clone(),
                },
            ))),
            _ => Err(PatchConduitDiagnosticError::ObservationTypeMismatch(
                observation.dimension_id.clone(),
            )),
        },
        OBS_CONTAINMENT => match &observation.value {
            WorkmanshipValue::Measurement { interval } => Ok(Some((
                DIM_CONTAINMENT,
                EngineeringFactValue::Measurement {
                    interval: interval.clone(),
                },
            ))),
            _ => Err(PatchConduitDiagnosticError::ObservationTypeMismatch(
                observation.dimension_id.clone(),
            )),
        },
        OBS_ACTIVE_LEAK => match &observation.value {
            WorkmanshipValue::Predicate { state } => Ok(Some((
                DIM_ACTIVE_LEAK,
                EngineeringFactValue::Predicate { state: *state },
            ))),
            _ => Err(PatchConduitDiagnosticError::ObservationTypeMismatch(
                observation.dimension_id.clone(),
            )),
        },
        _ => Ok(None),
    }
}

fn diagnostic_case(
    scenario: &PatchConduitScenario,
    pressure: &PatchConduitProcessContract,
    inspect: &PatchConduitProcessContract,
) -> Result<DiagnosticCase, PatchConduitDiagnosticError> {
    DiagnosticCase::new(
        DiagnosticCaseId::new(sid(
            "diagnostic-case:firstlight:patch-conduit:pressure-failure",
        )),
        &scenario.design,
        vec![
            DiagnosticHypothesis::new(
                DiagnosticHypothesisId::new(sid(HYPOTHESIS_COUPLING_LEAK)),
                vec![
                    criterion(CONSTRAINT_CONTAINMENT, KnownConstraintState::Unsatisfied),
                    criterion(CONSTRAINT_NO_ACTIVE_LEAK, KnownConstraintState::Unsatisfied),
                ],
            )?,
            DiagnosticHypothesis::new(
                DiagnosticHypothesisId::new(sid(HYPOTHESIS_RESTRICTION)),
                vec![
                    criterion(CONSTRAINT_PRESSURE_LOSS, KnownConstraintState::Unsatisfied),
                    criterion(CONSTRAINT_NO_ACTIVE_LEAK, KnownConstraintState::Satisfied),
                ],
            )?,
            DiagnosticHypothesis::new(
                DiagnosticHypothesisId::new(sid(HYPOTHESIS_MATERIAL)),
                vec![criterion(
                    CONSTRAINT_WATER_COMPATIBILITY,
                    KnownConstraintState::Unsatisfied,
                )],
            )?,
        ],
        vec![
            DiagnosticProbe::new(
                DiagnosticProbeId::new(sid(PROBE_VISUAL_LEAK)),
                inspect.spec.id.clone(),
                inspect.spec.revision,
                vec![constraint_id(CONSTRAINT_NO_ACTIVE_LEAK)],
                vec![sid(
                    "evidence-kind:firstlight:patch-conduit:leak-localization",
                )],
            )?,
            DiagnosticProbe::new(
                DiagnosticProbeId::new(sid(PROBE_REPEAT_PRESSURE)),
                pressure.spec.id.clone(),
                pressure.spec.revision,
                vec![
                    constraint_id(CONSTRAINT_PRESSURE_LOSS),
                    constraint_id(CONSTRAINT_CONTAINMENT),
                ],
                vec![sid(
                    "evidence-kind:firstlight:patch-conduit:pressure-observation",
                )],
            )?,
        ],
    )
    .map_err(Into::into)
}

fn criterion(id: &str, required: KnownConstraintState) -> DiagnosticCriterion {
    DiagnosticCriterion {
        constraint_id: constraint_id(id),
        required,
    }
}

fn constraint_id(value: &str) -> FunctionalConstraintId {
    FunctionalConstraintId::new(sid(value))
}

fn bypass_binding() -> Result<DesignBinding, ConstraintError> {
    DesignBinding::new(vec![RoleBinding {
        role_id: DesignRoleId::new(sid(REPAIR_ROLE)),
        subject: bypass_subject(),
    }])
}

fn bypass_subject() -> FunctionalSubject {
    FunctionalSubject {
        kind: FunctionalSubjectKind::Assembly,
        id: sid(BYPASS_SUBJECT),
    }
}

fn material_compatibility_fact() -> Result<EngineeringFact, ConstraintError> {
    Ok(EngineeringFact {
        id: EngineeringFactId::new(sid(
            "engineering-fact:firstlight:bypass-material-compatibility",
        )),
        subject: bypass_subject(),
        dimension_id: sid(DIM_WATER_COMPATIBILITY),
        value: EngineeringFactValue::Category {
            value_id: sid("material:compatibility:service-water"),
        },
        evidence: EngineeringEvidenceRef::new(
            sid("authority:firstlight:materials-inspection"),
            sid("evidence:firstlight:bypass-material-compatibility"),
            2,
            "digest:firstlight:bypass-material-compatibility:2",
        )?,
    })
}

fn reference_pressure_process_evidence(
    contract: &PatchConduitProcessContract,
) -> Result<ProcessEvidence, PatchConduitDiagnosticError> {
    let mut workpiece = Workpiece::new(
        WorkpieceId::new(sid(
            "workpiece:firstlight:patch-conduit:diagnostic-reference",
        )),
        vec![MatterBinding::new(
            sid("matter:firstlight:reference-physical"),
            sid("allocation:firstlight:patch-conduit:diagnostic-reference"),
            2,
            "digest:matter:firstlight:patch-conduit:diagnostic-reference:2",
        )?],
    )?;
    workpiece.transition(WorkpieceLifecycle::Available)?;

    let capabilities = contract
        .spec
        .required_capabilities
        .iter()
        .enumerate()
        .map(|(index, requirement)| CapabilityEvidence {
            capability_id: requirement.capability_id.clone(),
            available_value: requirement.minimum_value,
            evidence_id: sid(format!(
                "capability-evidence:firstlight:diagnostic-pressure:{index}"
            )),
        })
        .collect::<Vec<_>>();

    let execution_id = ProcessExecutionId::new(sid(
        "process-execution:firstlight:patch-conduit:diagnostic-pressure-test",
    ));
    let mut execution =
        ProcessExecution::begin(execution_id, &contract.spec, &[&workpiece], &capabilities)?;
    Ok(execution.complete(
        sid("matter:firstlight:reference-physical"),
        sid("process-evidence:firstlight:patch-conduit:diagnostic-pressure-test"),
        3,
        "digest:process:firstlight:patch-conduit:diagnostic-pressure-test:3",
        workpiece.matter_bindings.clone(),
    )?)
}

fn reference_pressure_observations(
    process: &ProcessEvidence,
    leak_state: Option<ObservationState>,
) -> Result<WorkmanshipVector, WorkmanshipError> {
    let mut vector = WorkmanshipVector::new(
        process.execution_id.clone(),
        process.spec_id.clone(),
        process.spec_revision,
    );
    vector.record(measurement_observation(
        "pressure-loss",
        OBS_PRESSURE_LOSS,
        250,
        290,
        5,
    )?)?;
    // Required containment starts at 350 kPa; this observation is therefore a
    // known functional failure even though the PressureTest process completed.
    vector.record(measurement_observation(
        "containment",
        OBS_CONTAINMENT,
        275,
        320,
        5,
    )?)?;
    if let Some(state) = leak_state {
        vector.record(WorkmanshipObservation {
            id: WorkmanshipObservationId::new(sid(
                "observation:firstlight:patch-conduit:active-leak",
            )),
            dimension_id: sid(OBS_ACTIVE_LEAK),
            value: WorkmanshipValue::Predicate { state },
            evidence: workmanship_evidence("active-leak", 4)?,
        })?;
    }
    Ok(vector)
}

fn measurement_observation(
    suffix: &str,
    dimension: &str,
    lower: i64,
    upper: i64,
    resolution: u64,
) -> Result<WorkmanshipObservation, WorkmanshipError> {
    Ok(WorkmanshipObservation {
        id: WorkmanshipObservationId::new(sid(format!(
            "observation:firstlight:patch-conduit:{suffix}"
        ))),
        dimension_id: sid(dimension),
        value: WorkmanshipValue::Measurement {
            interval: MeasurementInterval::new(lower, upper, resolution)?,
        },
        evidence: workmanship_evidence(suffix, 4)?,
    })
}

fn workmanship_evidence(
    suffix: &str,
    revision: u64,
) -> Result<WorkmanshipEvidenceRef, WorkmanshipError> {
    WorkmanshipEvidenceRef::new(
        sid("authority:firstlight:pressure-test-instrumentation"),
        sid(format!("evidence:firstlight:patch-conduit:{suffix}")),
        revision,
        format!("digest:instrument:firstlight:patch-conduit:{suffix}:{revision}"),
    )
}

fn hypothesis_ids(assessment: &DiagnosticAssessment, outcome: HypothesisOutcome) -> Vec<String> {
    assessment
        .hypotheses
        .iter()
        .filter(|hypothesis| hypothesis.outcome == outcome)
        .map(|hypothesis| hypothesis.hypothesis_id.stable_id().as_str().to_owned())
        .collect()
}

fn plan_step(value: &str) -> PlanStepId {
    PlanStepId::new(sid(value))
}

fn sid(value: impl Into<String>) -> StableId {
    StableId::parse(value).expect("Patch Conduit diagnostic identifiers must be valid")
}

#[derive(Debug)]
pub enum PatchConduitDiagnosticError {
    Scenario(String),
    ExecutionProfile(String),
    MissingPressureContract,
    MissingInspectContract,
    WrongProcessKind {
        expected: ProcessKind,
        actual: ProcessKind,
    },
    ProcessNotCompleted,
    MalformedProcessEvidence,
    ObservationContextMismatch,
    ObservationTypeMismatch(StableId),
    NoMappedPressureObservations,
    InvalidMaterialFact,
    Constraint(ConstraintError),
    Diagnostic(DiagnosticError),
    Workmanship(WorkmanshipError),
    Fabrication(FabricationError),
    Process(ProcessError),
}

impl From<ConstraintError> for PatchConduitDiagnosticError {
    fn from(error: ConstraintError) -> Self {
        Self::Constraint(error)
    }
}

impl From<DiagnosticError> for PatchConduitDiagnosticError {
    fn from(error: DiagnosticError) -> Self {
        Self::Diagnostic(error)
    }
}

impl From<WorkmanshipError> for PatchConduitDiagnosticError {
    fn from(error: WorkmanshipError) -> Self {
        Self::Workmanship(error)
    }
}

impl From<FabricationError> for PatchConduitDiagnosticError {
    fn from(error: FabricationError) -> Self {
        Self::Fabrication(error)
    }
}

impl From<ProcessError> for PatchConduitDiagnosticError {
    fn from(error: ProcessError) -> Self {
        Self::Process(error)
    }
}

impl fmt::Display for PatchConduitDiagnosticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scenario(error) => write!(formatter, "Patch Conduit scenario failed: {error}"),
            Self::ExecutionProfile(error) => {
                write!(formatter, "Patch Conduit execution profile failed: {error}")
            }
            Self::MissingPressureContract => write!(formatter, "pressure-test contract is missing"),
            Self::MissingInspectContract => write!(formatter, "inspection contract is missing"),
            Self::WrongProcessKind { expected, actual } => write!(
                formatter,
                "diagnostic evidence expected {expected:?} process, got {actual:?}"
            ),
            Self::ProcessNotCompleted => {
                write!(
                    formatter,
                    "diagnostic observations require completed process evidence"
                )
            }
            Self::MalformedProcessEvidence => {
                write!(
                    formatter,
                    "pressure-test evidence lacks physical before/after evidence"
                )
            }
            Self::ObservationContextMismatch => write!(
                formatter,
                "workmanship observations do not bind the exact pressure-test execution/spec revision"
            ),
            Self::ObservationTypeMismatch(id) => write!(
                formatter,
                "pressure-test observation {id} uses the wrong evidence value type"
            ),
            Self::NoMappedPressureObservations => {
                write!(
                    formatter,
                    "no mapped pressure-test observations were supplied"
                )
            }
            Self::InvalidMaterialFact => write!(
                formatter,
                "material fact does not bind the Patch Conduit bypass compatibility dimension"
            ),
            Self::Constraint(error) => fmt::Display::fmt(error, formatter),
            Self::Diagnostic(error) => fmt::Display::fmt(error, formatter),
            Self::Workmanship(error) => fmt::Display::fmt(error, formatter),
            Self::Fabrication(error) => fmt::Display::fmt(error, formatter),
            Self::Process(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl Error for PatchConduitDiagnosticError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Constraint(error) => Some(error),
            Self::Diagnostic(error) => Some(error),
            Self::Workmanship(error) => Some(error),
            Self::Fabrication(error) => Some(error),
            Self::Process(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_pressure_test_can_still_be_a_known_functional_failure() {
        let report = run_reference_pressure_diagnostics().unwrap();
        assert!(report.pressure_process_completed);
        assert!(!report.known_failure_all_satisfied);
        assert!(
            report
                .supported_hypotheses
                .contains(&HYPOTHESIS_COUPLING_LEAK.to_owned())
        );
        assert!(
            report
                .contradicted_hypotheses
                .contains(&HYPOTHESIS_RESTRICTION.to_owned())
        );
        assert!(
            report
                .contradicted_hypotheses
                .contains(&HYPOTHESIS_MATERIAL.to_owned())
        );
    }

    #[test]
    fn missing_leak_localization_keeps_fault_unresolved_and_suggests_probe() {
        let report = run_reference_pressure_diagnostics().unwrap();
        assert!(
            report
                .unresolved_without_leak_localization
                .contains(&HYPOTHESIS_COUPLING_LEAK.to_owned())
        );
        assert!(
            report
                .suggested_probes_without_leak_localization
                .contains(&PROBE_VISUAL_LEAK.to_owned())
        );
    }

    #[test]
    fn observations_from_another_execution_are_rejected() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let profile = PatchConduitExecutionProfile::compile(&scenario).unwrap();
        let pressure = profile
            .catalog
            .process_for_step(&plan_step(PRESSURE_STEP))
            .unwrap();
        let evidence = reference_pressure_process_evidence(pressure).unwrap();
        let mut vector =
            reference_pressure_observations(&evidence, Some(ObservationState::Present)).unwrap();
        vector.execution_id = ProcessExecutionId::new(sid("process-execution:other"));

        assert!(matches!(
            engineering_facts_from_pressure_test(
                &evidence,
                &vector,
                material_compatibility_fact().unwrap(),
            ),
            Err(PatchConduitDiagnosticError::ObservationContextMismatch)
        ));
    }

    #[test]
    fn report_contains_no_rank_confidence_or_selected_diagnosis() {
        let json = serde_json::to_string(&run_reference_pressure_diagnostics().unwrap()).unwrap();
        for forbidden in [
            "\"rank\"",
            "\"score\"",
            "\"confidence\"",
            "\"probability\"",
            "\"selected_diagnosis\"",
            "\"authorized\"",
            "\"commissioned\"",
        ] {
            assert!(!json.contains(forbidden), "unexpected field {forbidden}");
        }
    }
}

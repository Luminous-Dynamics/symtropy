// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Headless Patch Conduit v2 repair-discovery composition for Firstlight Basin.
//!
//! This module intentionally separates two questions:
//! 1. **Can a candidate repair satisfy the hydraulic/material requirements?**
//!    F7/F8 answer this from externally supplied engineering evidence.
//! 2. **How can that candidate be installed?**
//!    F10/C5 describe known fabrication plans and temporary-work sequencing.
//!
//! A plan existing in this catalog never makes its candidate valid. Likewise a
//! verified functional candidate is not technically commissioned or civically
//! authorized merely because the solver accepted its evidence.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_construction::{TemporaryWorkContract, TemporaryWorkError, TemporaryWorkId, TemporaryWorkKind};
use symtropy_fabrication::{
    ConstraintError, ConstraintPredicate, DesignRole, DesignRoleId, EngineeringEvidenceRef,
    EngineeringFact, EngineeringFactId, EngineeringFactSet, EngineeringFactValue,
    FabricationPlan, FabricationPlanId, FunctionalConstraint, FunctionalConstraintId,
    FunctionalDesign, FunctionalDesignId, FunctionalSubject, FunctionalSubjectKind,
    MeasurementInterval, ObservationState, PlanDependency, PlanError, PlanStep, PlanStepId,
    ProcessSpecId, RoleCandidateSet, SubstitutionError, SubstitutionResult, SubstitutionSolution,
    SubstitutionSolver, WorkmanshipError, WorkpieceId,
};
use symtropy_game_state::StableId;

const REPAIR_ROLE: &str = "role:patch-conduit:repair-assembly";
const DIM_PRESSURE_LOSS: &str = "engineering:pressure-loss-pa";
const DIM_CONTAINMENT: &str = "engineering:pressure-containment-kpa";
const DIM_WATER_COMPATIBILITY: &str = "engineering:water-service-compatibility";
const DIM_ACTIVE_LEAK: &str = "engineering:active-leak";

const STANDARD_SUBJECT: &str = "assembly:patch-conduit:standard-banded";
const SALVAGED_SUBJECT: &str = "assembly:patch-conduit:salvaged-sleeve";
const BYPASS_SUBJECT: &str = "assembly:patch-conduit:emergency-bypass";
const DEGRADED_SUBJECT: &str = "assembly:patch-conduit:degraded-hose";

/// One known installation procedure associated with a candidate functional
/// assembly. There is deliberately no `RepairKind`, `preferred`, or quest-solution
/// flag: functional eligibility is derived from F8 results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchConduitApproach {
    pub subject: FunctionalSubject,
    pub plan: FabricationPlan,
    pub temporary_works: Vec<TemporaryWorkContract>,
}

/// Reference Firstlight knowledge for the waterworks repair slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchConduitScenario {
    pub design: FunctionalDesign,
    approaches: Vec<PatchConduitApproach>,
}

impl PatchConduitScenario {
    /// Builds the authored knowledge catalog. It does not fabricate any of the
    /// referenced workpieces or assert that any candidate currently works.
    pub fn canonical() -> Result<Self, PatchConduitError> {
        let design = functional_design()?;
        let approaches = vec![
            standard_banded_approach()?,
            salvaged_sleeve_approach()?,
            emergency_bypass_approach(BYPASS_SUBJECT, "serviceable")?,
            emergency_bypass_approach(DEGRADED_SUBJECT, "degraded")?,
        ];

        Ok(Self { design, approaches })
    }

    pub fn approaches(&self) -> &[PatchConduitApproach] {
        &self.approaches
    }

    pub fn approach_for_subject(&self, subject: &FunctionalSubject) -> Option<&PatchConduitApproach> {
        self.approaches.iter().find(|approach| &approach.subject == subject)
    }

    /// Evaluates every catalog candidate through F8. The solver receives no
    /// hint about which approach is standard, salvaged, improvised, cheap, or
    /// desirable; candidate identities and evidence are the only inputs.
    pub fn evaluate(
        &self,
        facts: &EngineeringFactSet,
    ) -> Result<SubstitutionResult, PatchConduitError> {
        let candidate_set = RoleCandidateSet {
            role_id: DesignRoleId::new(sid(REPAIR_ROLE)),
            candidates: self
                .approaches
                .iter()
                .map(|approach| approach.subject.clone())
                .collect(),
        };
        Ok(SubstitutionSolver::new(64)?.solve(
            &self.design,
            &[candidate_set],
            facts,
        )?)
    }

    /// Resolves only F8-verified solutions to their known installation plans.
    /// A solver result from another design/revision is rejected rather than
    /// silently being interpreted as Patch Conduit evidence.
    pub fn verified_approaches<'a>(
        &'a self,
        result: &SubstitutionResult,
    ) -> Result<Vec<&'a PatchConduitApproach>, PatchConduitError> {
        self.resolve_solutions(&result.verified)
    }

    pub fn conditional_approaches<'a>(
        &'a self,
        result: &SubstitutionResult,
    ) -> Result<Vec<&'a PatchConduitApproach>, PatchConduitError> {
        self.resolve_solutions(&result.conditional)
    }

    fn resolve_solutions<'a>(
        &'a self,
        solutions: &[SubstitutionSolution],
    ) -> Result<Vec<&'a PatchConduitApproach>, PatchConduitError> {
        let mut resolved = Vec::with_capacity(solutions.len());
        for solution in solutions {
            if solution.evaluation.design_id != self.design.id
                || solution.evaluation.design_revision != self.design.revision
            {
                return Err(PatchConduitError::EvaluationDesignMismatch {
                    expected_id: self.design.id.clone(),
                    expected_revision: self.design.revision,
                    actual_id: solution.evaluation.design_id.clone(),
                    actual_revision: solution.evaluation.design_revision,
                });
            }
            let subject = solution
                .binding
                .assignments()
                .iter()
                .find(|binding| binding.role_id == DesignRoleId::new(sid(REPAIR_ROLE)))
                .map(|binding| &binding.subject)
                .ok_or(PatchConduitError::MissingRepairBinding)?;
            let approach = self
                .approach_for_subject(subject)
                .ok_or_else(|| PatchConduitError::UnknownApproach(subject.id.clone()))?;
            resolved.push(approach);
        }
        Ok(resolved)
    }
}

/// Deterministic reference engineering evidence used by the headless proof.
/// Real gameplay should replace these records with observations from the live
/// hydraulic/material/diagnostic authorities.
pub fn patch_conduit_reference_facts() -> Result<EngineeringFactSet, PatchConduitError> {
    let mut facts = Vec::new();

    add_candidate_facts(
        &mut facts,
        STANDARD_SUBJECT,
        "standard",
        (70, 95),
        (900, 980),
        "material:compatibility:potable-water",
        ObservationState::Absent,
    )?;
    add_candidate_facts(
        &mut facts,
        SALVAGED_SUBJECT,
        "salvaged",
        (115, 155),
        (620, 700),
        "material:compatibility:service-water",
        ObservationState::Absent,
    )?;
    add_candidate_facts(
        &mut facts,
        BYPASS_SUBJECT,
        "bypass",
        (245, 295),
        (380, 430),
        "material:compatibility:service-water",
        ObservationState::Absent,
    )?;
    // A plan exists for this material, but F8 must still reject it from the
    // reference conditions. This is intentional proof that plan existence is
    // not a recipe-style validity flag.
    add_candidate_facts(
        &mut facts,
        DEGRADED_SUBJECT,
        "degraded",
        (480, 550),
        (230, 280),
        "material:compatibility:unknown-polymer",
        ObservationState::Present,
    )?;

    Ok(EngineeringFactSet::new(facts)?)
}

fn functional_design() -> Result<FunctionalDesign, PatchConduitError> {
    let role_id = DesignRoleId::new(sid(REPAIR_ROLE));
    Ok(FunctionalDesign::new(
        FunctionalDesignId::new(sid("functional-design:firstlight:patch-conduit-v2")),
        2,
        vec![DesignRole {
            id: role_id.clone(),
            subject_kind: FunctionalSubjectKind::Assembly,
        }],
        vec![
            FunctionalConstraint {
                id: FunctionalConstraintId::new(sid("constraint:patch-conduit:pressure-loss")),
                role_id: role_id.clone(),
                dimension_id: sid(DIM_PRESSURE_LOSS),
                predicate: ConstraintPredicate::MeasurementWithin {
                    lower: 0,
                    upper: 350,
                },
            },
            FunctionalConstraint {
                id: FunctionalConstraintId::new(sid("constraint:patch-conduit:containment")),
                role_id: role_id.clone(),
                dimension_id: sid(DIM_CONTAINMENT),
                predicate: ConstraintPredicate::MeasurementWithin {
                    lower: 350,
                    upper: 2_500,
                },
            },
            FunctionalConstraint {
                id: FunctionalConstraintId::new(sid("constraint:patch-conduit:water-compatibility")),
                role_id: role_id.clone(),
                dimension_id: sid(DIM_WATER_COMPATIBILITY),
                predicate: ConstraintPredicate::CategoryOneOf {
                    accepted: vec![
                        sid("material:compatibility:potable-water"),
                        sid("material:compatibility:service-water"),
                    ],
                },
            },
            FunctionalConstraint {
                id: FunctionalConstraintId::new(sid("constraint:patch-conduit:no-active-leak")),
                role_id,
                dimension_id: sid(DIM_ACTIVE_LEAK),
                predicate: ConstraintPredicate::PredicateIs {
                    required: ObservationState::Absent,
                },
            },
        ],
    )?)
}

fn standard_banded_approach() -> Result<PatchConduitApproach, PatchConduitError> {
    let conduit = wp("ruptured-line");
    let band = wp("standard-band");
    let brace = wp("field-brace-standard");
    let plan = FabricationPlan::new(
        FabricationPlanId::new(sid("fabrication-plan:firstlight:patch-conduit:standard-banded")),
        1,
        vec![
            step("standard-clean", vec![conduit.clone(), band.clone()])?,
            step("standard-install-brace", vec![conduit.clone(), brace.clone()])?,
            step("standard-align", vec![conduit.clone(), band.clone()])?,
            step("standard-clamp", vec![conduit.clone(), band.clone()])?,
            step("standard-seal", vec![conduit.clone(), band])?,
            step("standard-pressure-test", vec![conduit.clone()])?,
            step("standard-remove-brace", vec![conduit, brace.clone()])?,
        ],
        chain(&[
            "standard-clean",
            "standard-install-brace",
            "standard-align",
            "standard-clamp",
            "standard-seal",
            "standard-pressure-test",
            "standard-remove-brace",
        ])?,
    )?;
    let temporary = TemporaryWorkContract::new(
        TemporaryWorkId::new(sid("temporary-work:patch-conduit:standard-brace")),
        &plan,
        TemporaryWorkKind::Brace,
        sid("purpose:patch-conduit:maintain-alignment"),
        vec![brace],
        plan_step_id("standard-install-brace"),
        vec![
            plan_step_id("standard-align"),
            plan_step_id("standard-clamp"),
            plan_step_id("standard-seal"),
            plan_step_id("standard-pressure-test"),
        ],
        Some(plan_step_id("standard-remove-brace")),
    )?;

    Ok(PatchConduitApproach {
        subject: assembly_subject(STANDARD_SUBJECT),
        plan,
        temporary_works: vec![temporary],
    })
}

fn salvaged_sleeve_approach() -> Result<PatchConduitApproach, PatchConduitError> {
    let conduit = wp("ruptured-line");
    let sleeve = wp("salvaged-sleeve");
    let brace = wp("field-brace-salvage");
    let plan = FabricationPlan::new(
        FabricationPlanId::new(sid("fabrication-plan:firstlight:patch-conduit:salvaged-sleeve")),
        1,
        vec![
            step("salvage-inspect", vec![sleeve.clone()])?,
            step("salvage-clean", vec![conduit.clone(), sleeve.clone()])?,
            step("salvage-install-brace", vec![conduit.clone(), brace.clone()])?,
            step("salvage-align", vec![conduit.clone(), sleeve.clone()])?,
            step("salvage-clamp", vec![conduit.clone(), sleeve.clone()])?,
            step("salvage-seal", vec![conduit.clone(), sleeve])?,
            step("salvage-pressure-test", vec![conduit.clone()])?,
            step("salvage-remove-brace", vec![conduit, brace.clone()])?,
        ],
        chain(&[
            "salvage-inspect",
            "salvage-clean",
            "salvage-install-brace",
            "salvage-align",
            "salvage-clamp",
            "salvage-seal",
            "salvage-pressure-test",
            "salvage-remove-brace",
        ])?,
    )?;
    let temporary = TemporaryWorkContract::new(
        TemporaryWorkId::new(sid("temporary-work:patch-conduit:salvage-brace")),
        &plan,
        TemporaryWorkKind::Brace,
        sid("purpose:patch-conduit:maintain-alignment"),
        vec![brace],
        plan_step_id("salvage-install-brace"),
        vec![
            plan_step_id("salvage-align"),
            plan_step_id("salvage-clamp"),
            plan_step_id("salvage-seal"),
            plan_step_id("salvage-pressure-test"),
        ],
        Some(plan_step_id("salvage-remove-brace")),
    )?;

    Ok(PatchConduitApproach {
        subject: assembly_subject(SALVAGED_SUBJECT),
        plan,
        temporary_works: vec![temporary],
    })
}

fn emergency_bypass_approach(
    subject_id: &str,
    suffix: &str,
) -> Result<PatchConduitApproach, PatchConduitError> {
    let conduit = wp("ruptured-line");
    let hose = wp(&format!("{suffix}-bypass-hose"));
    let upstream = wp(&format!("{suffix}-adapter-upstream"));
    let downstream = wp(&format!("{suffix}-adapter-downstream"));
    let install_name = format!("{suffix}-bypass-install");
    let test_name = format!("{suffix}-bypass-pressure-test");

    let plan = FabricationPlan::new(
        FabricationPlanId::new(sid(format!(
            "fabrication-plan:firstlight:patch-conduit:{suffix}-bypass"
        ))),
        1,
        vec![
            step(
                &install_name,
                vec![
                    conduit.clone(),
                    hose.clone(),
                    upstream.clone(),
                    downstream.clone(),
                ],
            )?,
            step(
                &test_name,
                vec![conduit, hose.clone(), upstream.clone(), downstream.clone()],
            )?,
        ],
        vec![PlanDependency::new(
            plan_step_id(&install_name),
            plan_step_id(&test_name),
        )?],
    )?;

    let temporary = TemporaryWorkContract::new(
        TemporaryWorkId::new(sid(format!("temporary-work:patch-conduit:{suffix}-bypass"))),
        &plan,
        TemporaryWorkKind::Bypass,
        sid("purpose:patch-conduit:restore-temporary-flow"),
        vec![hose, upstream, downstream],
        plan_step_id(&install_name),
        vec![plan_step_id(&test_name)],
        None,
    )?;

    Ok(PatchConduitApproach {
        subject: assembly_subject(subject_id),
        plan,
        temporary_works: vec![temporary],
    })
}

fn step(name: &str, workpieces: Vec<WorkpieceId>) -> Result<PlanStep, PlanError> {
    PlanStep::new(
        plan_step_id(name),
        ProcessSpecId::new(sid(format!("process-spec:firstlight:patch-conduit:{name}"))),
        1,
        workpieces,
        Vec::new(),
        vec![sid(format!("evidence-kind:firstlight:patch-conduit:{name}"))],
    )
}

fn chain(names: &[&str]) -> Result<Vec<PlanDependency>, PlanError> {
    names
        .windows(2)
        .map(|pair| PlanDependency::new(plan_step_id(pair[0]), plan_step_id(pair[1])))
        .collect()
}

fn plan_step_id(name: &str) -> PlanStepId {
    PlanStepId::new(sid(format!("step:firstlight:patch-conduit:{name}")))
}

fn wp(name: &str) -> WorkpieceId {
    WorkpieceId::new(sid(format!("workpiece:firstlight:patch-conduit:{name}")))
}

fn assembly_subject(value: &str) -> FunctionalSubject {
    FunctionalSubject {
        kind: FunctionalSubjectKind::Assembly,
        id: sid(value),
    }
}

#[allow(clippy::too_many_arguments)]
fn add_candidate_facts(
    facts: &mut Vec<EngineeringFact>,
    subject_id: &str,
    suffix: &str,
    pressure_loss: (i64, i64),
    containment: (i64, i64),
    compatibility: &str,
    leak_state: ObservationState,
) -> Result<(), PatchConduitError> {
    let subject = assembly_subject(subject_id);
    facts.push(measurement_fact(
        subject.clone(),
        suffix,
        "pressure-loss",
        DIM_PRESSURE_LOSS,
        pressure_loss.0,
        pressure_loss.1,
    )?);
    facts.push(measurement_fact(
        subject.clone(),
        suffix,
        "containment",
        DIM_CONTAINMENT,
        containment.0,
        containment.1,
    )?);
    facts.push(EngineeringFact {
        id: EngineeringFactId::new(sid(format!("fact:patch-conduit:{suffix}:compatibility"))),
        subject: subject.clone(),
        dimension_id: sid(DIM_WATER_COMPATIBILITY),
        value: EngineeringFactValue::Category {
            value_id: sid(compatibility),
        },
        evidence: evidence_ref(suffix, "compatibility")?,
    });
    facts.push(EngineeringFact {
        id: EngineeringFactId::new(sid(format!("fact:patch-conduit:{suffix}:leak"))),
        subject,
        dimension_id: sid(DIM_ACTIVE_LEAK),
        value: EngineeringFactValue::Predicate { state: leak_state },
        evidence: evidence_ref(suffix, "leak")?,
    });
    Ok(())
}

fn measurement_fact(
    subject: FunctionalSubject,
    suffix: &str,
    fact_suffix: &str,
    dimension: &str,
    lower: i64,
    upper: i64,
) -> Result<EngineeringFact, PatchConduitError> {
    Ok(EngineeringFact {
        id: EngineeringFactId::new(sid(format!(
            "fact:patch-conduit:{suffix}:{fact_suffix}"
        ))),
        subject,
        dimension_id: sid(dimension),
        value: EngineeringFactValue::Measurement {
            interval: MeasurementInterval::new(lower, upper, 5)?,
        },
        evidence: evidence_ref(suffix, fact_suffix)?,
    })
}

fn evidence_ref(suffix: &str, kind: &str) -> Result<EngineeringEvidenceRef, ConstraintError> {
    EngineeringEvidenceRef::new(
        sid("authority:firstlight:waterworks-diagnostics"),
        sid(format!("evidence:patch-conduit:{suffix}:{kind}")),
        1,
        format!("digest:patch-conduit:{suffix}:{kind}:1"),
    )
}

fn sid(value: impl Into<String>) -> StableId {
    StableId::parse(value).expect("Patch Conduit authored stable identifiers must be valid")
}

#[derive(Debug)]
pub enum PatchConduitError {
    Constraint(ConstraintError),
    Workmanship(WorkmanshipError),
    Plan(PlanError),
    TemporaryWork(TemporaryWorkError),
    Substitution(SubstitutionError),
    EvaluationDesignMismatch {
        expected_id: FunctionalDesignId,
        expected_revision: u64,
        actual_id: FunctionalDesignId,
        actual_revision: u64,
    },
    MissingRepairBinding,
    UnknownApproach(StableId),
}

impl From<ConstraintError> for PatchConduitError {
    fn from(error: ConstraintError) -> Self {
        Self::Constraint(error)
    }
}

impl From<WorkmanshipError> for PatchConduitError {
    fn from(error: WorkmanshipError) -> Self {
        Self::Workmanship(error)
    }
}

impl From<PlanError> for PatchConduitError {
    fn from(error: PlanError) -> Self {
        Self::Plan(error)
    }
}

impl From<TemporaryWorkError> for PatchConduitError {
    fn from(error: TemporaryWorkError) -> Self {
        Self::TemporaryWork(error)
    }
}

impl From<SubstitutionError> for PatchConduitError {
    fn from(error: SubstitutionError) -> Self {
        Self::Substitution(error)
    }
}

impl fmt::Display for PatchConduitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Constraint(error) => write!(formatter, "Patch Conduit constraint failed: {error}"),
            Self::Workmanship(error) => write!(formatter, "Patch Conduit measurement failed: {error}"),
            Self::Plan(error) => write!(formatter, "Patch Conduit plan failed: {error}"),
            Self::TemporaryWork(error) => write!(formatter, "Patch Conduit temporary work failed: {error}"),
            Self::Substitution(error) => write!(formatter, "Patch Conduit substitution failed: {error}"),
            Self::EvaluationDesignMismatch {
                expected_id,
                expected_revision,
                actual_id,
                actual_revision,
            } => write!(
                formatter,
                "Patch Conduit expected evaluation {expected_id}@{expected_revision}, got {actual_id}@{actual_revision}"
            ),
            Self::MissingRepairBinding => write!(formatter, "Patch Conduit solver result has no repair-role binding"),
            Self::UnknownApproach(id) => write!(formatter, "Patch Conduit solver returned unknown approach {id}"),
        }
    }
}

impl Error for PatchConduitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Constraint(error) => Some(error),
            Self::Workmanship(error) => Some(error),
            Self::Plan(error) => Some(error),
            Self::TemporaryWork(error) => Some(error),
            Self::Substitution(error) => Some(error),
            Self::EvaluationDesignMismatch { .. }
            | Self::MissingRepairBinding
            | Self::UnknownApproach(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solution_ids(solutions: &[SubstitutionSolution]) -> Vec<StableId> {
        solutions
            .iter()
            .map(|solution| solution.binding.assignments()[0].subject.id.clone())
            .collect()
    }

    #[test]
    fn reference_evidence_yields_three_verified_approaches_without_rank() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let result = scenario.evaluate(&patch_conduit_reference_facts().unwrap()).unwrap();

        assert_eq!(result.verified.len(), 3);
        assert_eq!(result.rejected_combinations, 1);
        assert!(result.conditional.is_empty());
        let ids = solution_ids(&result.verified);
        assert!(ids.contains(&sid(STANDARD_SUBJECT)));
        assert!(ids.contains(&sid(SALVAGED_SUBJECT)));
        assert!(ids.contains(&sid(BYPASS_SUBJECT)));
        assert!(!ids.contains(&sid(DEGRADED_SUBJECT)));

        let resolved = scenario.verified_approaches(&result).unwrap();
        assert_eq!(resolved.len(), 3);
    }

    #[test]
    fn candidate_catalog_order_does_not_choose_the_solution_order() {
        let mut forward = PatchConduitScenario::canonical().unwrap();
        let facts = patch_conduit_reference_facts().unwrap();
        let first = solution_ids(&forward.evaluate(&facts).unwrap().verified);
        forward.approaches.reverse();
        let reversed = solution_ids(&forward.evaluate(&facts).unwrap().verified);
        assert_eq!(first, reversed);
    }

    #[test]
    fn missing_bypass_leak_evidence_makes_it_conditional_not_verified() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let reference = patch_conduit_reference_facts().unwrap();
        let filtered = EngineeringFactSet::new(
            reference
                .facts()
                .iter()
                .filter(|fact| {
                    !(fact.subject.id == sid(BYPASS_SUBJECT)
                        && fact.dimension_id == sid(DIM_ACTIVE_LEAK))
                })
                .cloned()
                .collect(),
        )
        .unwrap();
        let result = scenario.evaluate(&filtered).unwrap();

        let verified = solution_ids(&result.verified);
        let conditional = solution_ids(&result.conditional);
        assert!(!verified.contains(&sid(BYPASS_SUBJECT)));
        assert!(conditional.contains(&sid(BYPASS_SUBJECT)));
    }

    #[test]
    fn plan_existence_does_not_make_degraded_hose_a_solution() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        assert!(scenario
            .approach_for_subject(&assembly_subject(DEGRADED_SUBJECT))
            .is_some());
        let result = scenario.evaluate(&patch_conduit_reference_facts().unwrap()).unwrap();
        assert!(!solution_ids(&result.verified).contains(&sid(DEGRADED_SUBJECT)));
        assert_eq!(result.rejected_combinations, 1);
    }

    #[test]
    fn standard_and_salvage_plans_use_braces_while_bypass_is_temporary_flow_work() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let standard = scenario
            .approach_for_subject(&assembly_subject(STANDARD_SUBJECT))
            .unwrap();
        let salvaged = scenario
            .approach_for_subject(&assembly_subject(SALVAGED_SUBJECT))
            .unwrap();
        let bypass = scenario
            .approach_for_subject(&assembly_subject(BYPASS_SUBJECT))
            .unwrap();

        assert_eq!(standard.temporary_works[0].kind, TemporaryWorkKind::Brace);
        assert!(standard.temporary_works[0].removal_step_id.is_some());
        assert_eq!(salvaged.temporary_works[0].kind, TemporaryWorkKind::Brace);
        assert!(salvaged.temporary_works[0].removal_step_id.is_some());
        assert_eq!(bypass.temporary_works[0].kind, TemporaryWorkKind::Bypass);
        assert!(bypass.temporary_works[0].removal_step_id.is_none());
    }

    #[test]
    fn scenario_serialization_contains_no_quest_rank_or_authority_oracle() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let json = serde_json::to_string(&scenario).unwrap();
        for forbidden in [
            "\"rank\"",
            "\"score\"",
            "\"preferred\"",
            "\"quest_solution\"",
            "\"authorized\"",
            "\"commissioned\"",
            "\"technical_ready\"",
        ] {
            assert!(!json.contains(forbidden), "unexpected field {forbidden}");
        }
    }
}

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Pure, dry-run replay of a factory reconfiguration from last accepted output
//! in source configuration A to first accepted output in target configuration B.
//! No machine control, scheduler, solver, network, filesystem or release authority.

use std::collections::BTreeSet;

pub const SCHEMA_VERSION: &str = "factory-reconfiguration/0.1";

const PHASES: [Phase; 9] = [
    Phase::DrainWip, Phase::SafeStop, Phase::Isolate, Phase::ChangeModules,
    Phase::LoadConfiguration, Phase::CalibrateAndStaff, Phase::Commission,
    Phase::FirstArticle, Phase::StableRestart,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactRef { pub id: String, pub generation: String, pub digest: String }

impl ExactRef {
    pub fn new(id: &str, generation: &str, digit: char) -> Self {
        Self { id: id.to_owned(), generation: generation.to_owned(), digest: digit.to_string().repeat(64) }
    }
    fn valid(&self) -> bool {
        !self.id.trim().is_empty() && !self.generation.trim().is_empty()
            && self.digest.len() == 64
            && self.digest.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase { DrainWip, SafeStop, Isolate, ChangeModules, LoadConfiguration, CalibrateAndStaff, Commission, FirstArticle, StableRestart }
impl Phase {
    fn index(self) -> usize {
        match self {
            Self::DrainWip => 0, Self::SafeStop => 1, Self::Isolate => 2,
            Self::ChangeModules => 3, Self::LoadConfiguration => 4,
            Self::CalibrateAndStaff => 5, Self::Commission => 6,
            Self::FirstArticle => 7, Self::StableRestart => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome { Completed, Failed, Unknown }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArticleState { NotApplicable, Accepted, Rejected, Held }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub wip_and_lots_disposed: bool,
    pub genealogy_complete: bool,
    pub safe_stop: bool,
    pub electrical_isolation: bool,
    pub pressure_fluid_isolation: bool,
    pub thermal_chemical_isolation: bool,
    pub module_interfaces_compatible: bool,
    pub utility_interfaces_compatible: bool,
    pub segregation_satisfied: bool,
    pub resources_not_double_allocated: bool,
    pub recipe_current: bool,
    pub product_revision_matches: bool,
    pub controller_configuration_current: bool,
    pub tooling_valid: bool,
    pub calibration_current: bool,
    pub qualified_staff_available: bool,
    pub commissioning_passed: bool,
    pub safety_interlocks_passed: bool,
    pub process_limits_checked: bool,
    pub qualified_inspector_available: bool,
    pub inspection_complete: bool,
    pub article_state: ArticleState,
    pub stable_service: bool,
    pub rollback_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub id: String,
    pub phase: Phase,
    pub outcome: StepOutcome,
    pub elapsed_ticks: u64,
    pub evidence: Evidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub schema_version: String,
    pub plan: ExactRef,
    pub source: ExactRef,
    pub target: ExactRef,
    pub product_revision: ExactRef,
    pub process_revision: ExactRef,
    pub time_basis: ExactRef,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub current_source: ExactRef,
    pub current_target: ExactRef,
    pub accepted_product_revision: ExactRef,
    pub accepted_process_revision: ExactRef,
    pub accepted_time_basis: ExactRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    AcceptedSynthetic, Blocked, Aborted, RollbackRequired, CommissioningFailed,
    FirstArticleRejected, TraceabilityBroken, UnknownOutcome, Reject,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalAcceptance { Unresolved }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    SchemaUnsupported, MalformedReference, SourceMismatch, TargetMismatch,
    ProductRevisionMismatch, ProcessRevisionMismatch, TimeBasisMismatch,
    DuplicateStepId, InvalidStepOrder, MissingPhase, MutationAfterUnknown,
    UnknownOutcomeRequiresReconciliation, WipDispositionIncomplete,
    GenealogyIncomplete, SafeStopMissing, ElectricalIsolationMissing,
    PressureFluidIsolationMissing, ThermalChemicalIsolationMissing,
    ModuleInterfaceIncompatible, UtilityInterfaceIncompatible,
    SegregationViolation, ResourceDoubleAllocated, RecipeStale,
    ProductRevisionStale, ControllerConfigurationStale, ToolingInvalid,
    CalibrationStale, QualifiedStaffMissing, CommissioningFailed,
    InterlockFailed, ProcessLimitsUnchecked, InspectorUnavailable,
    FirstArticleInspectionIncomplete, FirstArticleRejected, FirstArticleHeld,
    StableServiceMissing, DurationOverflow, RollbackVerified, RollbackNotVerified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub disposition: Disposition,
    pub reasons: Vec<Reason>,
    pub completed_steps: usize,
    pub elapsed_ticks: u64,
    pub first_article_accepted: bool,
    pub physical_acceptance: PhysicalAcceptance,
    pub production_authorized: bool,
    pub further_mutation_authorized: bool,
}

fn fail(disposition: Disposition, reason: Reason) -> Result<(), (Disposition, Reason)> {
    Err((disposition, reason))
}

fn check_evidence(step: &Step) -> Result<(), (Disposition, Reason)> {
    let e = &step.evidence;
    if !e.genealogy_complete {
        return fail(Disposition::TraceabilityBroken, Reason::GenealogyIncomplete);
    }
    let missing = match step.phase {
        Phase::DrainWip => !e.wip_and_lots_disposed,
        Phase::SafeStop => !e.safe_stop,
        Phase::Isolate => {
            if !e.electrical_isolation { return fail(Disposition::Blocked, Reason::ElectricalIsolationMissing); }
            if !e.pressure_fluid_isolation { return fail(Disposition::Blocked, Reason::PressureFluidIsolationMissing); }
            if !e.thermal_chemical_isolation { return fail(Disposition::Blocked, Reason::ThermalChemicalIsolationMissing); }
            false
        }
        Phase::ChangeModules => {
            if !e.module_interfaces_compatible { return fail(Disposition::Blocked, Reason::ModuleInterfaceIncompatible); }
            if !e.utility_interfaces_compatible { return fail(Disposition::Blocked, Reason::UtilityInterfaceIncompatible); }
            if !e.segregation_satisfied { return fail(Disposition::Blocked, Reason::SegregationViolation); }
            if !e.resources_not_double_allocated { return fail(Disposition::Blocked, Reason::ResourceDoubleAllocated); }
            false
        }
        Phase::LoadConfiguration => {
            if !e.recipe_current { return fail(Disposition::Blocked, Reason::RecipeStale); }
            if !e.product_revision_matches { return fail(Disposition::Blocked, Reason::ProductRevisionStale); }
            if !e.controller_configuration_current { return fail(Disposition::Blocked, Reason::ControllerConfigurationStale); }
            false
        }
        Phase::CalibrateAndStaff => {
            if !e.tooling_valid { return fail(Disposition::Blocked, Reason::ToolingInvalid); }
            if !e.calibration_current { return fail(Disposition::Blocked, Reason::CalibrationStale); }
            if !e.qualified_staff_available { return fail(Disposition::Blocked, Reason::QualifiedStaffMissing); }
            false
        }
        Phase::Commission => {
            if !e.commissioning_passed { return fail(Disposition::CommissioningFailed, Reason::CommissioningFailed); }
            if !e.safety_interlocks_passed { return fail(Disposition::Blocked, Reason::InterlockFailed); }
            if !e.process_limits_checked { return fail(Disposition::Blocked, Reason::ProcessLimitsUnchecked); }
            false
        }
        Phase::FirstArticle => {
            if !e.qualified_inspector_available { return fail(Disposition::Blocked, Reason::InspectorUnavailable); }
            if !e.inspection_complete { return fail(Disposition::Blocked, Reason::FirstArticleInspectionIncomplete); }
            match e.article_state {
                ArticleState::Accepted => false,
                ArticleState::Rejected => return fail(Disposition::FirstArticleRejected, Reason::FirstArticleRejected),
                ArticleState::Held | ArticleState::NotApplicable => return fail(Disposition::Blocked, Reason::FirstArticleHeld),
            }
        }
        Phase::StableRestart => !e.stable_service,
    };
    if missing {
        let reason = match step.phase {
            Phase::DrainWip => Reason::WipDispositionIncomplete,
            Phase::SafeStop => Reason::SafeStopMissing,
            Phase::StableRestart => Reason::StableServiceMissing,
            _ => Reason::WipDispositionIncomplete,
        };
        return fail(Disposition::Blocked, reason);
    }
    Ok(())
}

/// Pure deterministic replay. AcceptedSynthetic never implies physical release.
pub fn replay(plan: &Plan, context: &Context) -> Receipt {
    let mut reasons = Vec::new();
    if plan.schema_version != SCHEMA_VERSION { reasons.push(Reason::SchemaUnsupported); }
    let refs = [
        &plan.plan, &plan.source, &plan.target, &plan.product_revision,
        &plan.process_revision, &plan.time_basis, &context.current_source,
        &context.current_target, &context.accepted_product_revision,
        &context.accepted_process_revision, &context.accepted_time_basis,
    ];
    if refs.iter().any(|r| !r.valid()) { reasons.push(Reason::MalformedReference); }
    if plan.source != context.current_source { reasons.push(Reason::SourceMismatch); }
    if plan.target != context.current_target { reasons.push(Reason::TargetMismatch); }
    if plan.product_revision != context.accepted_product_revision { reasons.push(Reason::ProductRevisionMismatch); }
    if plan.process_revision != context.accepted_process_revision { reasons.push(Reason::ProcessRevisionMismatch); }
    if plan.time_basis != context.accepted_time_basis { reasons.push(Reason::TimeBasisMismatch); }

    let mut ids = BTreeSet::new();
    let mut phases = Vec::new();
    for step in &plan.steps {
        if step.id.trim().is_empty() || !ids.insert(step.id.as_str()) { reasons.push(Reason::DuplicateStepId); }
        phases.push(step.phase);
    }
    if phases.windows(2).any(|window| window[0].index() >= window[1].index()) {
        reasons.push(Reason::InvalidStepOrder);
    }
    let terminal_index = plan.steps.iter().position(|step| step.outcome != StepOutcome::Completed);
    if let Some(index) = terminal_index {
        if index + 1 < plan.steps.len() {
            reasons.push(if plan.steps[index].outcome == StepOutcome::Unknown {
                Reason::MutationAfterUnknown
            } else {
                Reason::InvalidStepOrder
            });
        }
    } else if plan.steps.len() != PHASES.len() {
        reasons.push(Reason::MissingPhase);
    }

    let structural_block_only = reasons.len() == 1 && reasons[0] == Reason::MissingPhase;
    let mut disposition = if reasons.is_empty() {
        Disposition::AcceptedSynthetic
    } else if structural_block_only {
        Disposition::Blocked
    } else {
        Disposition::Reject
    };
    let mut elapsed_ticks = 0_u64;
    let mut completed_steps = 0;
    if reasons.is_empty() {
        for (index, step) in plan.steps.iter().enumerate() {
            match step.outcome {
                StepOutcome::Unknown => {
                    disposition = if index + 1 < plan.steps.len() { Disposition::Reject } else { Disposition::UnknownOutcome };
                    reasons.push(if index + 1 < plan.steps.len() { Reason::MutationAfterUnknown } else { Reason::UnknownOutcomeRequiresReconciliation });
                    break;
                }
                StepOutcome::Failed => {
                    if step.phase == Phase::Commission {
                        disposition = Disposition::CommissioningFailed;
                        reasons.push(Reason::CommissioningFailed);
                    } else if step.phase == Phase::FirstArticle {
                        disposition = Disposition::FirstArticleRejected;
                        reasons.push(Reason::FirstArticleRejected);
                    } else if step.evidence.rollback_verified {
                        disposition = Disposition::Aborted;
                        reasons.push(Reason::RollbackVerified);
                    } else {
                        disposition = Disposition::RollbackRequired;
                        reasons.push(Reason::RollbackNotVerified);
                    }
                    break;
                }
                StepOutcome::Completed => {
                    if let Err((d, r)) = check_evidence(step) {
                        disposition = d;
                        reasons.push(r);
                        break;
                    }
                    match elapsed_ticks.checked_add(step.elapsed_ticks) {
                        Some(total) => elapsed_ticks = total,
                        None => {
                            disposition = Disposition::Reject;
                            reasons.push(Reason::DurationOverflow);
                            break;
                        }
                    }
                    completed_steps += 1;
                }
            }
        }
    }
    reasons.sort();
    reasons.dedup();
    let first_article_accepted = disposition == Disposition::AcceptedSynthetic
        && plan.steps.iter().any(|s| s.phase == Phase::FirstArticle && s.evidence.article_state == ArticleState::Accepted);
    Receipt {
        disposition,
        reasons,
        completed_steps,
        elapsed_ticks,
        first_article_accepted,
        physical_acceptance: PhysicalAcceptance::Unresolved,
        production_authorized: false,
        further_mutation_authorized: !plan.steps.iter().any(|step| step.outcome == StepOutcome::Unknown),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(id: &str, generation: &str, digit: char) -> ExactRef {
        ExactRef::new(id, generation, &digit.to_string().repeat(64))
    }

    fn evidence(phase: Phase) -> Evidence {
        Evidence {
            wip_and_lots_disposed: true, genealogy_complete: true, safe_stop: true,
            electrical_isolation: true, pressure_fluid_isolation: true, thermal_chemical_isolation: true,
            module_interfaces_compatible: true, utility_interfaces_compatible: true,
            segregation_satisfied: true, resources_not_double_allocated: true,
            recipe_current: true, product_revision_matches: true, controller_configuration_current: true,
            tooling_valid: true, calibration_current: true, qualified_staff_available: true,
            commissioning_passed: true, safety_interlocks_passed: true, process_limits_checked: true,
            qualified_inspector_available: true, inspection_complete: true,
            article_state: if phase == Phase::FirstArticle { ArticleState::Accepted } else { ArticleState::NotApplicable },
            stable_service: true, rollback_verified: false,
        }
    }

    fn plan() -> Plan {
        Plan {
            schema_version: SCHEMA_VERSION.to_owned(),
            plan: r("RECONFIG-PLAN", "p1", '1'),
            source: r("CONFIG-A", "a1", '2'), target: r("CONFIG-B", "b1", '3'),
            product_revision: r("PRODUCT-7", "v7", '4'),
            process_revision: r("ROUTE-4", "r4", '5'),
            time_basis: r("SYNTHETIC-TICKS", "t1", '6'),
            steps: PHASES.iter().enumerate().map(|(i, phase)| Step {
                id: format!("STEP-{i}"), phase: *phase, outcome: StepOutcome::Completed,
                elapsed_ticks: (i as u64) + 1, evidence: evidence(*phase),
            }).collect(),
        }
    }

    fn context() -> Context {
        Context {
            current_source: r("CONFIG-A", "a1", '2'),
            current_target: r("CONFIG-B", "b1", '3'),
            accepted_product_revision: r("PRODUCT-7", "v7", '4'),
            accepted_process_revision: r("ROUTE-4", "r4", '5'),
            accepted_time_basis: r("SYNTHETIC-TICKS", "t1", '6'),
        }
    }

    fn has(receipt: &Receipt, reason: Reason) { assert!(receipt.reasons.contains(&reason), "{:?}", receipt.reasons); }

    #[test]
    fn complete_trace_is_synthetic_only() {
        let out = replay(&plan(), &context());
        assert_eq!(out.disposition, Disposition::AcceptedSynthetic);
        assert_eq!(out.completed_steps, 9);
        assert_eq!(out.elapsed_ticks, 45);
        assert!(out.first_article_accepted);
        assert_eq!(out.physical_acceptance, PhysicalAcceptance::Unresolved);
        assert!(!out.production_authorized);
        assert!(out.further_mutation_authorized);
    }

    #[test]
    fn stale_source_configuration_rejects() {
        let mut p = plan(); p.source = r("CONFIG-OLD", "a0", '2');
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Reject); has(&out, Reason::SourceMismatch);
    }

    #[test]
    fn stale_target_configuration_rejects() {
        let mut p = plan(); p.target = r("CONFIG-B", "b0", '3');
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Reject); has(&out, Reason::TargetMismatch);
    }

    #[test]
    fn stale_product_revision_rejects() {
        let mut p = plan(); p.product_revision = r("PRODUCT-6", "v6", '4');
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Reject); has(&out, Reason::ProductRevisionMismatch);
    }

    #[test]
    fn stale_process_route_rejects() {
        let mut p = plan(); p.process_revision = r("ROUTE-OLD", "r3", '5');
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Reject); has(&out, Reason::ProcessRevisionMismatch);
    }

    #[test]
    fn missing_phase_rejects() {
        let mut p = plan(); p.steps.remove(3);
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Reject); has(&out, Reason::MissingPhase);
    }

    #[test]
    fn duplicate_step_id_rejects() {
        let mut p = plan(); p.steps[1].id = p.steps[0].id.clone();
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Reject); has(&out, Reason::DuplicateStepId);
    }

    #[test]
    fn out_of_order_phases_reject() {
        let mut p = plan(); p.steps.swap(2, 3);
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Reject); has(&out, Reason::InvalidStepOrder);
    }

    #[test]
    fn unaccounted_wip_blocks() {
        let mut p = plan(); p.steps[0].evidence.wip_and_lots_disposed = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::WipDispositionIncomplete);
    }

    #[test]
    fn broken_genealogy_is_terminal() {
        let mut p = plan(); p.steps[3].evidence.genealogy_complete = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::TraceabilityBroken); has(&out, Reason::GenealogyIncomplete);
    }

    #[test]
    fn missing_safe_stop_blocks() {
        let mut p = plan(); p.steps[1].evidence.safe_stop = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::SafeStopMissing);
    }

    #[test]
    fn missing_hazard_isolation_blocks() {
        let mut p = plan(); p.steps[2].evidence.thermal_chemical_isolation = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::ThermalChemicalIsolationMissing);
    }

    #[test]
    fn incompatible_module_interface_blocks() {
        let mut p = plan(); p.steps[3].evidence.module_interfaces_compatible = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::ModuleInterfaceIncompatible);
    }

    #[test]
    fn segregation_violation_blocks() {
        let mut p = plan(); p.steps[3].evidence.segregation_satisfied = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::SegregationViolation);
    }

    #[test]
    fn stale_recipe_blocks() {
        let mut p = plan(); p.steps[4].evidence.recipe_current = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::RecipeStale);
    }

    #[test]
    fn stale_calibration_blocks() {
        let mut p = plan(); p.steps[5].evidence.calibration_current = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::CalibrationStale);
    }

    #[test]
    fn missing_qualified_staff_blocks() {
        let mut p = plan(); p.steps[5].evidence.qualified_staff_available = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::QualifiedStaffMissing);
    }

    #[test]
    fn failed_commissioning_cannot_reach_first_article() {
        let mut p = plan(); p.steps[6].outcome = StepOutcome::Failed; p.steps.truncate(7);
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::CommissioningFailed); has(&out, Reason::CommissioningFailed);
        assert!(!out.first_article_accepted);
    }

    #[test]
    fn rejected_first_article_prevents_target_acceptance() {
        let mut p = plan(); p.steps[7].evidence.article_state = ArticleState::Rejected;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::FirstArticleRejected); has(&out, Reason::FirstArticleRejected);
    }

    #[test]
    fn held_first_article_blocks_target_acceptance() {
        let mut p = plan(); p.steps[7].evidence.article_state = ArticleState::Held;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::FirstArticleHeld);
    }

    #[test]
    fn unknown_outcome_blocks_further_mutation() {
        let mut p = plan(); p.steps[4].outcome = StepOutcome::Unknown; p.steps.truncate(5);
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::UnknownOutcome);
        has(&out, Reason::UnknownOutcomeRequiresReconciliation);
        assert!(!out.further_mutation_authorized);
    }

    #[test]
    fn mutation_after_unknown_outcome_rejects() {
        let mut p = plan(); p.steps[4].outcome = StepOutcome::Unknown;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Reject); has(&out, Reason::MutationAfterUnknown);
        assert!(!out.further_mutation_authorized);
    }

    #[test]
    fn failed_step_requires_rollback_or_returns_rollback_required() {
        let mut p = plan(); p.steps[3].outcome = StepOutcome::Failed; p.steps.truncate(4);
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::RollbackRequired); has(&out, Reason::RollbackNotVerified);
    }

    #[test]
    fn verified_rollback_marks_transition_aborted() {
        let mut p = plan(); p.steps[3].outcome = StepOutcome::Failed; p.steps[3].evidence.rollback_verified = true; p.steps.truncate(4);
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Aborted); has(&out, Reason::RollbackVerified);
    }

    #[test]
    fn malformed_digest_rejects() {
        let mut p = plan(); p.plan.digest = "bad".to_owned();
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Reject); has(&out, Reason::MalformedReference);
    }

    #[test]
    fn elapsed_tick_overflow_rejects() {
        let mut p = plan(); p.steps[0].elapsed_ticks = u64::MAX; p.steps[1].elapsed_ticks = 1;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Reject); has(&out, Reason::DurationOverflow);
    }

    #[test]
    fn missing_inspector_blocks_first_article() {
        let mut p = plan(); p.steps[7].evidence.qualified_inspector_available = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::InspectorUnavailable);
    }

    #[test]
    fn stable_service_required_after_first_article() {
        let mut p = plan(); p.steps[8].evidence.stable_service = false;
        let out = replay(&p, &context());
        assert_eq!(out.disposition, Disposition::Blocked); has(&out, Reason::StableServiceMissing);
    }
}

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Read-only admission checks between source-aware recipe tasks and exact
//! appliance-module capability revisions.
//!
//! This module verifies declared task/module compatibility only. It does not
//! perform automatic module selection, control appliances, certify safety,
//! establish physical fitment, or validate food quality.

use crate::KitchenScheduleReceiptV1;
use crate::action_ontology::{ActionGroupV1, validate_action_ontology_binding};
use crate::module_capability::{
    KitchenModuleCapabilityV1, ModuleCapabilityClaimStatusV1, ModuleEvidenceClassV1,
    validate_kitchen_module_capability,
};
use crate::power::{PowerConstrainedPlanV1, validate_power_envelope};
use crate::recipe_graph::{
    ExactRecipeQuantityV1, RecipeQuantityUnitV1, RecipeTaskGraphV1, RecipeTaskNodeV1,
    recipe_graph_to_kitchen_plan, validate_recipe_task_graph,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const KITCHEN_CAPABILITY_ADMISSION_SCHEMA_VERSION: u32 = 1;
pub const KITCHEN_CAPABILITY_ADMISSION_CLAIM_CEILING: &str = "Task-to-module declaration consistency only; not physical interoperability, safety certification, food-contact approval, measured performance, or actuator authorization.";

const NANOGRAMS_PER_MILLIGRAM: u128 = 1_000_000;
const NANOGRAMS_PER_GRAM: u128 = 1_000_000_000;
const NANOGRAMS_PER_KILOGRAM: u128 = 1_000_000_000_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenTaskModuleBindingV1 {
    pub task_id: String,
    pub module_id: String,
    pub module_revision: String,
    pub capability_id: String,
    pub input_form_ref: String,
    pub output_form_ref: String,
    /// Must already be present in the task's KitchenTaskV1.resource_ids.
    pub scheduled_resource_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenCapabilityAdmissionRequestV1 {
    pub schema_version: u32,
    pub request_id: String,
    pub recipe: RecipeTaskGraphV1,
    pub modules: Vec<KitchenModuleCapabilityV1>,
    pub bindings: Vec<KitchenTaskModuleBindingV1>,
    pub power_plan: PowerConstrainedPlanV1,
    pub schedule: KitchenScheduleReceiptV1,
    pub claim_ceiling: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenCapabilityDispositionV1 {
    WithinDeclaredEnvelope,
    Indeterminate,
    Blocked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KitchenCapabilityIssueCodeV1 {
    UnsupportedSchemaVersion,
    MissingRequiredValue,
    InvalidClaimCeiling,
    InvalidRecipeGraph,
    DuplicateModuleRevision,
    DuplicateTaskBinding,
    UnknownTaskBinding,
    MissingTaskBinding,
    UnknownModule,
    StaleModuleRevision,
    InvalidModuleCapability,
    UnknownCapability,
    MissingActionOntology,
    UnmappedAction,
    InvalidActionOntology,
    ActionGroupMismatch,
    MissingInputForm,
    UnsupportedInputForm,
    MissingOutputForm,
    UnsupportedOutputForm,
    MissingScheduledResource,
    ScheduledResourceMismatch,
    MissingBatchMass,
    UnsupportedBatchMassUnit,
    InvalidBatchMassScale,
    BatchMassArithmeticOverflow,
    BatchBelowMinimum,
    BatchAboveMaximum,
    MissingMaximumRunTime,
    MaximumRunTimeExceeded,
    PowerPlanRecipeMismatch,
    InvalidPowerSchedule,
    MissingTaskPowerProfile,
    MissingModuleContinuousPowerLimit,
    MissingModulePeakPowerLimit,
    MissingModuleSurgeDuration,
    ModulePeakPowerExceeded,
    ModuleSurgeDurationExceeded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenCapabilityIssueV1 {
    pub code: KitchenCapabilityIssueCodeV1,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenTaskCapabilityAssessmentV1 {
    pub task_id: String,
    pub disposition: KitchenCapabilityDispositionV1,
    pub module_id: Option<String>,
    pub module_revision: Option<String>,
    pub capability_id: Option<String>,
    pub action_group: Option<ActionGroupV1>,
    pub claim_status: Option<ModuleCapabilityClaimStatusV1>,
    pub evidence_class: Option<ModuleEvidenceClassV1>,
    pub issues: Vec<KitchenCapabilityIssueV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenCapabilityAdmissionReportV1 {
    pub schema_version: u32,
    pub request_id: String,
    pub recipe_id: String,
    pub disposition: KitchenCapabilityDispositionV1,
    pub admitted: bool,
    pub request_issues: Vec<KitchenCapabilityIssueV1>,
    pub task_assessments: Vec<KitchenTaskCapabilityAssessmentV1>,
    pub claim_ceiling: String,
}

impl KitchenCapabilityAdmissionReportV1 {
    /// True only when every recipe task is within its declared module envelope
    /// and there are no request-level issues. This is not actuation authority.
    pub fn is_admitted(&self) -> bool {
        self.admitted
    }
}

fn add_issue(
    issues: &mut Vec<KitchenCapabilityIssueV1>,
    code: KitchenCapabilityIssueCodeV1,
    detail: impl Into<String>,
) {
    issues.push(KitchenCapabilityIssueV1 {
        code,
        detail: detail.into(),
    });
}

fn sort_issues(issues: &mut [KitchenCapabilityIssueV1]) {
    issues.sort_by(|left, right| {
        (left.code, left.detail.as_str()).cmp(&(right.code, right.detail.as_str()))
    });
}

fn disposition_from_issues(issues: &[KitchenCapabilityIssueV1]) -> KitchenCapabilityDispositionV1 {
    if issues.iter().any(|issue| {
        matches!(
            issue.code,
            KitchenCapabilityIssueCodeV1::DuplicateTaskBinding
                | KitchenCapabilityIssueCodeV1::UnknownTaskBinding
                | KitchenCapabilityIssueCodeV1::UnknownModule
                | KitchenCapabilityIssueCodeV1::StaleModuleRevision
                | KitchenCapabilityIssueCodeV1::InvalidModuleCapability
                | KitchenCapabilityIssueCodeV1::UnknownCapability
                | KitchenCapabilityIssueCodeV1::InvalidActionOntology
                | KitchenCapabilityIssueCodeV1::ActionGroupMismatch
                | KitchenCapabilityIssueCodeV1::UnsupportedInputForm
                | KitchenCapabilityIssueCodeV1::UnsupportedOutputForm
                | KitchenCapabilityIssueCodeV1::ScheduledResourceMismatch
                | KitchenCapabilityIssueCodeV1::BatchBelowMinimum
                | KitchenCapabilityIssueCodeV1::BatchAboveMaximum
                | KitchenCapabilityIssueCodeV1::BatchMassArithmeticOverflow
                | KitchenCapabilityIssueCodeV1::MaximumRunTimeExceeded
                | KitchenCapabilityIssueCodeV1::PowerPlanRecipeMismatch
                | KitchenCapabilityIssueCodeV1::InvalidPowerSchedule
                | KitchenCapabilityIssueCodeV1::ModulePeakPowerExceeded
                | KitchenCapabilityIssueCodeV1::ModuleSurgeDurationExceeded
        )
    }) {
        KitchenCapabilityDispositionV1::Blocked
    } else if !issues.is_empty() {
        KitchenCapabilityDispositionV1::Indeterminate
    } else {
        KitchenCapabilityDispositionV1::WithinDeclaredEnvelope
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BatchMassError {
    Missing,
    UnsupportedUnit,
    InvalidScale,
    Overflow,
}

/// Convert one explicit mass quantity into integer nanograms. The input
/// quantity is an integer coefficient plus a decimal scale; no floating-point
/// conversion or density assumption is used.
fn quantity_mass_nanograms(quantity: &ExactRecipeQuantityV1) -> Result<u128, BatchMassError> {
    if quantity.decimal_places > 6 {
        return Err(BatchMassError::InvalidScale);
    }
    let base_nanograms = match quantity.unit {
        RecipeQuantityUnitV1::Milligram => NANOGRAMS_PER_MILLIGRAM,
        RecipeQuantityUnitV1::Gram => NANOGRAMS_PER_GRAM,
        RecipeQuantityUnitV1::Kilogram => NANOGRAMS_PER_KILOGRAM,
        RecipeQuantityUnitV1::Millilitre
        | RecipeQuantityUnitV1::Litre
        | RecipeQuantityUnitV1::Piece
        | RecipeQuantityUnitV1::Count
        | RecipeQuantityUnitV1::Teaspoon
        | RecipeQuantityUnitV1::Tablespoon
        | RecipeQuantityUnitV1::Cup => return Err(BatchMassError::UnsupportedUnit),
    };
    let divisor = 10_u128
        .checked_pow(u32::from(quantity.decimal_places))
        .ok_or(BatchMassError::InvalidScale)?;
    if base_nanograms % divisor != 0 {
        return Err(BatchMassError::InvalidScale);
    }
    u128::from(quantity.value)
        .checked_mul(base_nanograms / divisor)
        .ok_or(BatchMassError::Overflow)
}

/// Sum all declared ingredient mass inputs for one task. A missing ingredient
/// list or any volume/count input makes mass comparison indeterminate.
fn task_batch_mass_nanograms(task: &RecipeTaskNodeV1) -> Result<u128, BatchMassError> {
    if task.ingredient_inputs.is_empty() {
        return Err(BatchMassError::Missing);
    }
    task.ingredient_inputs
        .iter()
        .try_fold(0_u128, |total, usage| {
            let amount = quantity_mass_nanograms(&usage.quantity)?;
            total.checked_add(amount).ok_or(BatchMassError::Overflow)
        })
}

/// Check one task power profile against the exact selected module's own
/// continuous, peak and transient limits. Household aggregate power is
/// independently verified at request level by the existing power validator.
fn assess_module_power(
    task: &RecipeTaskNodeV1,
    module: &KitchenModuleCapabilityV1,
    power_plan: &PowerConstrainedPlanV1,
    issues: &mut Vec<KitchenCapabilityIssueV1>,
) {
    let Some(profile) = power_plan
        .task_profiles
        .iter()
        .find(|profile| profile.task_id == task.schedule.task_id)
    else {
        add_issue(
            issues,
            KitchenCapabilityIssueCodeV1::MissingTaskPowerProfile,
            "task has no explicit power profile in the validated power plan",
        );
        return;
    };

    let Some(continuous_limit_w) = module.electrical.continuous_real_power_limit_w else {
        add_issue(
            issues,
            KitchenCapabilityIssueCodeV1::MissingModuleContinuousPowerLimit,
            "module has no declared continuous real-power limit",
        );
        return;
    };
    let Some(peak_limit_w) = module.electrical.peak_real_power_limit_w else {
        add_issue(
            issues,
            KitchenCapabilityIssueCodeV1::MissingModulePeakPowerLimit,
            "module has no declared peak real-power limit",
        );
        return;
    };

    let mut contiguous_surge_ms = 0_u64;
    let mut peak_issue_added = false;
    let mut surge_issue_added = false;
    let mut missing_surge_issue_added = false;
    for segment in &profile.segments {
        if segment.real_power_w > peak_limit_w && !peak_issue_added {
            add_issue(
                issues,
                KitchenCapabilityIssueCodeV1::ModulePeakPowerExceeded,
                "task power segment exceeds the exact module revision's peak real-power limit",
            );
            peak_issue_added = true;
        }

        if segment.real_power_w > continuous_limit_w {
            let duration_ms = segment
                .end_offset_ms
                .saturating_sub(segment.start_offset_ms);
            contiguous_surge_ms = contiguous_surge_ms.saturating_add(duration_ms);
            match module.electrical.maximum_surge_duration_ms {
                Some(maximum_ms) if contiguous_surge_ms > maximum_ms => {
                    if !surge_issue_added {
                        add_issue(
                            issues,
                            KitchenCapabilityIssueCodeV1::ModuleSurgeDurationExceeded,
                            "contiguous task demand above the module's continuous limit exceeds its declared maximum surge duration",
                        );
                        surge_issue_added = true;
                    }
                }
                Some(_) => {}
                None if segment.real_power_w <= peak_limit_w && !missing_surge_issue_added => {
                    add_issue(
                        issues,
                        KitchenCapabilityIssueCodeV1::MissingModuleSurgeDuration,
                        "module has no declared maximum surge duration for demand above its continuous limit",
                    );
                    missing_surge_issue_added = true;
                }
                None => {}
            }
        } else {
            contiguous_surge_ms = 0;
        }
    }
}

fn task_assessment(
    task: &RecipeTaskNodeV1,
    binding: &KitchenTaskModuleBindingV1,
    modules: &[KitchenModuleCapabilityV1],
    power_plan: &PowerConstrainedPlanV1,
) -> KitchenTaskCapabilityAssessmentV1 {
    let mut issues = Vec::new();
    let mut action_group = None;
    let mut claim_status = None;
    let mut evidence_class = None;

    if binding.module_id.trim().is_empty()
        || binding.module_revision.trim().is_empty()
        || binding.capability_id.trim().is_empty()
    {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::MissingRequiredValue,
            "module ID, exact revision and capability ID must be non-empty",
        );
    }

    let Some(ontology) = task.ontology_binding.as_ref() else {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::MissingActionOntology,
            "task has no explicit action-ontology binding; action-group inference is not performed",
        );
        sort_issues(&mut issues);
        return assessment_from(
            task,
            binding,
            action_group,
            claim_status,
            evidence_class,
            issues,
        );
    };

    let ontology_report = validate_action_ontology_binding(task.action_kind, ontology);
    if !ontology_report.is_valid() {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::InvalidActionOntology,
            "task action-ontology binding failed its existing validator",
        );
    } else if let Some(group) = ontology.mapping.group {
        action_group = Some(group);
    } else {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::UnmappedAction,
            "task action is explicitly unmapped; no capability match is inferred",
        );
        sort_issues(&mut issues);
        return assessment_from(
            task,
            binding,
            action_group,
            claim_status,
            evidence_class,
            issues,
        );
    }

    if issues
        .iter()
        .any(|issue| issue.code == KitchenCapabilityIssueCodeV1::InvalidActionOntology)
    {
        sort_issues(&mut issues);
        return assessment_from(
            task,
            binding,
            action_group,
            claim_status,
            evidence_class,
            issues,
        );
    }

    let exact_module = modules.iter().find(|module| {
        module.module_id == binding.module_id && module.module_revision == binding.module_revision
    });
    let Some(module) = exact_module else {
        let same_id_exists = modules
            .iter()
            .any(|module| module.module_id == binding.module_id);
        if same_id_exists {
            add_issue(
                &mut issues,
                KitchenCapabilityIssueCodeV1::StaleModuleRevision,
                "module ID exists, but the supplied revision does not match any inventory entry",
            );
        } else {
            add_issue(
                &mut issues,
                KitchenCapabilityIssueCodeV1::UnknownModule,
                "no module with the supplied exact ID and revision exists in the request inventory",
            );
        }
        sort_issues(&mut issues);
        return assessment_from(
            task,
            binding,
            action_group,
            claim_status,
            evidence_class,
            issues,
        );
    };

    let module_report = validate_kitchen_module_capability(module);
    if !module_report.is_valid() {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::InvalidModuleCapability,
            "selected module record failed the existing module-capability validator",
        );
        sort_issues(&mut issues);
        return assessment_from(
            task,
            binding,
            action_group,
            claim_status,
            evidence_class,
            issues,
        );
    }

    let capability = module
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == binding.capability_id);
    let Some(capability) = capability else {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::UnknownCapability,
            "capability ID does not exist on the exact selected module revision",
        );
        sort_issues(&mut issues);
        return assessment_from(
            task,
            binding,
            action_group,
            claim_status,
            evidence_class,
            issues,
        );
    };

    claim_status = Some(capability.claim_status);
    evidence_class = Some(capability.evidence.evidence_class);

    if Some(capability.action_group) != action_group {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::ActionGroupMismatch,
            "recipe task action group differs from the selected module capability",
        );
    }

    if binding.input_form_ref.trim().is_empty() {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::MissingInputForm,
            "binding input_form_ref must be explicit and non-empty",
        );
    } else if !capability
        .input_form_refs
        .iter()
        .any(|form| form == &binding.input_form_ref)
    {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::UnsupportedInputForm,
            "binding input form is not declared by the selected capability",
        );
    }

    if binding.output_form_ref.trim().is_empty() {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::MissingOutputForm,
            "binding output_form_ref must be explicit and non-empty",
        );
    } else if !capability
        .output_form_refs
        .iter()
        .any(|form| form == &binding.output_form_ref)
    {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::UnsupportedOutputForm,
            "binding output form is not declared by the selected capability",
        );
    }

    if binding.scheduled_resource_id.trim().is_empty() {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::MissingScheduledResource,
            "scheduled_resource_id must be explicit and non-empty",
        );
    } else if !task
        .schedule
        .resource_ids
        .iter()
        .any(|resource| resource == &binding.scheduled_resource_id)
    {
        add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::ScheduledResourceMismatch,
            "bound module resource is not present in the task's scheduled resource_ids",
        );
    }

    if capability.minimum_batch_mass_g.is_some() || capability.maximum_batch_mass_g.is_some() {
        match task_batch_mass_nanograms(task) {
            Ok(mass_ng) => {
                if let Some(minimum_g) = capability.minimum_batch_mass_g {
                    let minimum_ng = u128::from(minimum_g) * NANOGRAMS_PER_GRAM;
                    if mass_ng < minimum_ng {
                        add_issue(
                            &mut issues,
                            KitchenCapabilityIssueCodeV1::BatchBelowMinimum,
                            "declared task ingredient mass is below the capability minimum batch mass",
                        );
                    }
                }
                if let Some(maximum_g) = capability.maximum_batch_mass_g {
                    let maximum_ng = u128::from(maximum_g) * NANOGRAMS_PER_GRAM;
                    if mass_ng > maximum_ng {
                        add_issue(
                            &mut issues,
                            KitchenCapabilityIssueCodeV1::BatchAboveMaximum,
                            "declared task ingredient mass exceeds the capability maximum batch mass",
                        );
                    }
                }
            }
            Err(BatchMassError::Missing) => add_issue(
                &mut issues,
                KitchenCapabilityIssueCodeV1::MissingBatchMass,
                "capability has batch-mass limits but task ingredient mass is not declared",
            ),
            Err(BatchMassError::UnsupportedUnit) => add_issue(
                &mut issues,
                KitchenCapabilityIssueCodeV1::UnsupportedBatchMassUnit,
                "batch mass cannot be inferred from volume, piece or count units",
            ),
            Err(BatchMassError::InvalidScale) => add_issue(
                &mut issues,
                KitchenCapabilityIssueCodeV1::InvalidBatchMassScale,
                "ingredient mass quantity has an unsupported or non-exact decimal scale",
            ),
            Err(BatchMassError::Overflow) => add_issue(
                &mut issues,
                KitchenCapabilityIssueCodeV1::BatchMassArithmeticOverflow,
                "checked batch-mass arithmetic overflowed",
            ),
        }
    }

    match capability.maximum_continuous_run_ms {
        Some(maximum_ms) if task.schedule.duration_ms > maximum_ms => add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::MaximumRunTimeExceeded,
            "task duration exceeds the capability's maximum continuous run time",
        ),
        Some(_) => {}
        None => add_issue(
            &mut issues,
            KitchenCapabilityIssueCodeV1::MissingMaximumRunTime,
            "capability has no declared maximum continuous run time",
        ),
    }

    assess_module_power(task, module, power_plan, &mut issues);
    sort_issues(&mut issues);
    assessment_from(
        task,
        binding,
        action_group,
        claim_status,
        evidence_class,
        issues,
    )
}

fn assessment_from(
    task: &RecipeTaskNodeV1,
    binding: &KitchenTaskModuleBindingV1,
    action_group: Option<ActionGroupV1>,
    claim_status: Option<ModuleCapabilityClaimStatusV1>,
    evidence_class: Option<ModuleEvidenceClassV1>,
    issues: Vec<KitchenCapabilityIssueV1>,
) -> KitchenTaskCapabilityAssessmentV1 {
    KitchenTaskCapabilityAssessmentV1 {
        task_id: task.schedule.task_id.clone(),
        disposition: disposition_from_issues(&issues),
        module_id: Some(binding.module_id.clone()),
        module_revision: Some(binding.module_revision.clone()),
        capability_id: Some(binding.capability_id.clone()),
        action_group,
        claim_status,
        evidence_class,
        issues,
    }
}

/// Evaluate one explicit recipe-to-module binding per task. The inventory and
/// every reference are checked by exact ID/revision; no matching is inferred.
pub fn evaluate_kitchen_capability_admission(
    request: &KitchenCapabilityAdmissionRequestV1,
) -> KitchenCapabilityAdmissionReportV1 {
    let mut request_issues = Vec::new();

    if request.schema_version != KITCHEN_CAPABILITY_ADMISSION_SCHEMA_VERSION {
        add_issue(
            &mut request_issues,
            KitchenCapabilityIssueCodeV1::UnsupportedSchemaVersion,
            "unsupported kitchen capability admission schema version",
        );
    }
    if request.request_id.trim().is_empty() {
        add_issue(
            &mut request_issues,
            KitchenCapabilityIssueCodeV1::MissingRequiredValue,
            "request_id must be non-empty",
        );
    }
    if request.claim_ceiling != KITCHEN_CAPABILITY_ADMISSION_CLAIM_CEILING {
        add_issue(
            &mut request_issues,
            KitchenCapabilityIssueCodeV1::InvalidClaimCeiling,
            "request claim ceiling must match the fixed descriptive-only statement",
        );
    }

    let recipe_report = validate_recipe_task_graph(&request.recipe);
    if !recipe_report.is_valid() {
        add_issue(
            &mut request_issues,
            KitchenCapabilityIssueCodeV1::InvalidRecipeGraph,
            "recipe graph failed its existing source-aware graph validator",
        );
    }

    let power_plan_mismatch = recipe_graph_to_kitchen_plan(&request.recipe)
        .is_ok_and(|mapped_plan| request.power_plan.plan != mapped_plan);
    if power_plan_mismatch {
        add_issue(
            &mut request_issues,
            KitchenCapabilityIssueCodeV1::PowerPlanRecipeMismatch,
            "power plan tasks and scheduling fields must exactly equal the recipe graph's mapped plan",
        );
    }

    let power_report = validate_power_envelope(&request.power_plan, &request.schedule);
    if !power_report.is_valid() {
        add_issue(
            &mut request_issues,
            KitchenCapabilityIssueCodeV1::InvalidPowerSchedule,
            format!(
                "existing power-envelope verifier reported {} violation(s)",
                power_report.violations.len()
            ),
        );
    }

    let mut module_revisions = BTreeSet::new();
    for module in &request.modules {
        if !module_revisions.insert((module.module_id.as_str(), module.module_revision.as_str())) {
            add_issue(
                &mut request_issues,
                KitchenCapabilityIssueCodeV1::DuplicateModuleRevision,
                "module inventory contains a duplicate exact ID/revision pair",
            );
        }
    }

    let known_task_ids: BTreeSet<&str> = request
        .recipe
        .tasks
        .iter()
        .map(|task| task.schedule.task_id.as_str())
        .collect();
    let mut bindings_by_task: BTreeMap<&str, Vec<&KitchenTaskModuleBindingV1>> = BTreeMap::new();
    for binding in &request.bindings {
        if !known_task_ids.contains(binding.task_id.as_str()) {
            add_issue(
                &mut request_issues,
                KitchenCapabilityIssueCodeV1::UnknownTaskBinding,
                "binding references a task ID absent from the recipe graph",
            );
        }
        bindings_by_task
            .entry(binding.task_id.as_str())
            .or_default()
            .push(binding);
    }

    let mut task_assessments = Vec::new();
    for task in &request.recipe.tasks {
        match bindings_by_task.get(task.schedule.task_id.as_str()) {
            None => {
                let mut issues = Vec::new();
                add_issue(
                    &mut issues,
                    KitchenCapabilityIssueCodeV1::MissingTaskBinding,
                    "recipe task has no explicit module/capability binding",
                );
                task_assessments.push(KitchenTaskCapabilityAssessmentV1 {
                    task_id: task.schedule.task_id.clone(),
                    disposition: KitchenCapabilityDispositionV1::Indeterminate,
                    module_id: None,
                    module_revision: None,
                    capability_id: None,
                    action_group: task
                        .ontology_binding
                        .as_ref()
                        .and_then(|binding| binding.mapping.group),
                    claim_status: None,
                    evidence_class: None,
                    issues,
                });
            }
            Some(bindings) if bindings.len() > 1 => {
                let mut issues = Vec::new();
                add_issue(
                    &mut issues,
                    KitchenCapabilityIssueCodeV1::DuplicateTaskBinding,
                    "task has more than one module binding; V1 requires exactly one explicit binding",
                );
                task_assessments.push(KitchenTaskCapabilityAssessmentV1 {
                    task_id: task.schedule.task_id.clone(),
                    disposition: KitchenCapabilityDispositionV1::Blocked,
                    module_id: None,
                    module_revision: None,
                    capability_id: None,
                    action_group: task
                        .ontology_binding
                        .as_ref()
                        .and_then(|binding| binding.mapping.group),
                    claim_status: None,
                    evidence_class: None,
                    issues,
                });
            }
            Some(bindings) => task_assessments.push(task_assessment(
                task,
                bindings[0],
                &request.modules,
                &request.power_plan,
            )),
        }
    }

    sort_issues(&mut request_issues);
    task_assessments.sort_by(|left, right| left.task_id.cmp(&right.task_id));

    let task_disposition = if task_assessments
        .iter()
        .any(|assessment| assessment.disposition == KitchenCapabilityDispositionV1::Blocked)
    {
        KitchenCapabilityDispositionV1::Blocked
    } else if task_assessments
        .iter()
        .any(|assessment| assessment.disposition == KitchenCapabilityDispositionV1::Indeterminate)
    {
        KitchenCapabilityDispositionV1::Indeterminate
    } else {
        KitchenCapabilityDispositionV1::WithinDeclaredEnvelope
    };

    let disposition = if !request_issues.is_empty()
        || task_disposition == KitchenCapabilityDispositionV1::Blocked
    {
        KitchenCapabilityDispositionV1::Blocked
    } else {
        task_disposition
    };

    KitchenCapabilityAdmissionReportV1 {
        schema_version: KITCHEN_CAPABILITY_ADMISSION_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        recipe_id: request.recipe.recipe_id.clone(),
        disposition,
        admitted: disposition == KitchenCapabilityDispositionV1::WithinDeclaredEnvelope,
        request_issues,
        task_assessments,
        claim_ceiling: KITCHEN_CAPABILITY_ADMISSION_CLAIM_CEILING.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_ontology::rule_based_action_binding;
    use crate::recipe_graph::{RecipeActionKindV1, RecipeEvidenceClassV1};

    fn fixture_request() -> KitchenCapabilityAdmissionRequestV1 {
        let module_bundle: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/fixtures/kitchen-module-capability-v1.json"
        ))
        .expect("module fixture bundle must parse");
        let recipe_bundle: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/fixtures/recipe-task-graph-v1.json"
        ))
        .expect("recipe fixture bundle must parse");

        let module: KitchenModuleCapabilityV1 =
            serde_json::from_value(module_bundle["cases"][0]["module"].clone())
                .expect("positive synthetic module fixture must deserialize");
        let mut recipe: RecipeTaskGraphV1 =
            serde_json::from_value(recipe_bundle["cases"][0]["payload"].clone())
                .expect("positive synthetic recipe fixture must deserialize");

        recipe.tasks.truncate(1);
        recipe.tasks[0].ontology_binding = Some(rule_based_action_binding(
            RecipeActionKindV1::Chop,
            RecipeEvidenceClassV1::Declared,
        ));

        let power_plan = PowerConstrainedPlanV1 {
            schema_version: crate::power::POWER_CONSTRAINED_PLAN_SCHEMA_VERSION,
            plan: recipe_graph_to_kitchen_plan(&recipe)
                .expect("positive synthetic recipe graph must map to a plan"),
            envelope: crate::power::PowerEnvelopeV1 {
                continuous_real_power_limit_w: 1_500,
                surge_real_power_limit_w: 2_000,
                max_surge_duration_ms: 100,
                background_real_power_w: 100,
            },
            task_profiles: vec![crate::power::TaskPowerProfileV1 {
                task_id: "chop-onion".to_owned(),
                source_ref: "fixture:task-power-profile".to_owned(),
                evidence_class: crate::power::PowerProfileEvidenceClassV1::Simulated,
                segments: vec![crate::power::PowerSegmentV1 {
                    start_offset_ms: 0,
                    end_offset_ms: 30_000,
                    real_power_w: 400,
                }],
            }],
        };
        let schedule = crate::power::build_power_constrained_schedule(&power_plan)
            .expect("positive synthetic power plan must schedule")
            .schedule;

        KitchenCapabilityAdmissionRequestV1 {
            schema_version: KITCHEN_CAPABILITY_ADMISSION_SCHEMA_VERSION,
            request_id: "fixture:valid-chop-admission".to_owned(),
            recipe,
            modules: vec![module],
            bindings: vec![KitchenTaskModuleBindingV1 {
                task_id: "chop-onion".to_owned(),
                module_id: "fixture:prep-drive-01".to_owned(),
                module_revision: "rev-a".to_owned(),
                capability_id: "capability:chop".to_owned(),
                input_form_ref: "form:whole-produce".to_owned(),
                output_form_ref: "form:chopped-produce".to_owned(),
                scheduled_resource_id: "processor".to_owned(),
            }],
            power_plan,
            schedule,
            claim_ceiling: KITCHEN_CAPABILITY_ADMISSION_CLAIM_CEILING.to_owned(),
        }
    }

    fn disposition(
        request: &KitchenCapabilityAdmissionRequestV1,
    ) -> KitchenCapabilityDispositionV1 {
        evaluate_kitchen_capability_admission(request).disposition
    }

    #[test]
    fn explicit_revision_action_form_resource_batch_and_runtime_match() {
        let request = fixture_request();
        let report = evaluate_kitchen_capability_admission(&request);
        assert!(report.is_admitted());
        assert_eq!(
            report.disposition,
            KitchenCapabilityDispositionV1::WithinDeclaredEnvelope
        );
        assert_eq!(report.task_assessments.len(), 1);
        assert_eq!(
            report.task_assessments[0].evidence_class,
            Some(ModuleEvidenceClassV1::Declared)
        );
    }

    #[test]
    fn stale_module_revision_is_blocked() {
        let mut request = fixture_request();
        request.bindings[0].module_revision = "rev-b".to_owned();
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn action_group_mismatch_is_blocked() {
        let mut request = fixture_request();
        request.modules[0].capabilities[0].action_group = ActionGroupV1::Slice;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn unsupported_form_or_resource_is_blocked() {
        let mut request = fixture_request();
        request.bindings[0].input_form_ref = "form:unknown".to_owned();
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );

        let mut request = fixture_request();
        request.bindings[0].scheduled_resource_id = "unallocated-device".to_owned();
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn missing_or_unmapped_action_ontology_is_indeterminate() {
        let mut request = fixture_request();
        request.recipe.tasks[0].ontology_binding = None;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Indeterminate
        );

        let mut request = fixture_request();
        request.recipe.tasks[0].ontology_binding = Some(rule_based_action_binding(
            RecipeActionKindV1::Other,
            RecipeEvidenceClassV1::Modelled,
        ));
        request.recipe.tasks[0].action_kind = RecipeActionKindV1::Other;
        request.recipe.tasks[0].action_label = Some("unmapped manual action".to_owned());
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Indeterminate
        );
    }

    #[test]
    fn batch_mass_limits_are_checked_without_volume_or_count_inference() {
        let mut request = fixture_request();
        request.recipe.tasks[0].ingredient_inputs[0].quantity.value = 2_000;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );

        let mut request = fixture_request();
        request.recipe.tasks[0].ingredient_inputs[0].quantity.value = 20;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );

        let mut request = fixture_request();
        request.recipe.tasks[0].ingredient_inputs[0].quantity.unit =
            RecipeQuantityUnitV1::Millilitre;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Indeterminate
        );
    }

    #[test]
    fn exact_milligram_and_kilogram_quantities_are_supported() {
        let mut request = fixture_request();
        request.recipe.tasks[0].ingredient_inputs[0].quantity.value = 250_000;
        request.recipe.tasks[0].ingredient_inputs[0].quantity.unit =
            RecipeQuantityUnitV1::Milligram;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::WithinDeclaredEnvelope
        );

        let mut request = fixture_request();
        request.recipe.tasks[0].ingredient_inputs[0].quantity.value = 25;
        request.recipe.tasks[0].ingredient_inputs[0]
            .quantity
            .decimal_places = 2;
        request.recipe.tasks[0].ingredient_inputs[0].quantity.unit = RecipeQuantityUnitV1::Kilogram;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::WithinDeclaredEnvelope
        );
    }

    #[test]
    fn unknown_module_and_unknown_capability_are_blocked() {
        let mut request = fixture_request();
        request.bindings[0].module_id = "module:not-in-inventory".to_owned();
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );

        let mut request = fixture_request();
        request.bindings[0].capability_id = "capability:not-on-module".to_owned();
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn duplicate_module_revision_and_unknown_task_binding_block_request() {
        let mut request = fixture_request();
        request.modules.push(request.modules[0].clone());
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );

        let mut request = fixture_request();
        let mut extra = request.bindings[0].clone();
        extra.task_id = "task:not-in-recipe".to_owned();
        request.bindings.push(extra);
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn missing_form_or_batch_quantity_remains_indeterminate() {
        let mut request = fixture_request();
        request.bindings[0].input_form_ref.clear();
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Indeterminate
        );

        let mut request = fixture_request();
        request.recipe.tasks[0].ingredient_inputs.clear();
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Indeterminate
        );
    }

    #[test]
    fn measured_capability_evidence_requires_calibration() {
        let mut request = fixture_request();
        request.modules[0].capabilities[0].evidence.evidence_class =
            ModuleEvidenceClassV1::Measured;
        request.modules[0].capabilities[0].evidence.calibration_ref = None;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn missing_or_exceeded_run_time_is_not_admitted() {
        let mut request = fixture_request();
        request.modules[0].capabilities[0].maximum_continuous_run_ms = None;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Indeterminate
        );

        let mut request = fixture_request();
        request.modules[0].capabilities[0].maximum_continuous_run_ms = Some(10_000);
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn missing_and_duplicate_bindings_are_not_admitted() {
        let mut request = fixture_request();
        request.bindings.clear();
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Indeterminate
        );

        let mut request = fixture_request();
        request.bindings.push(request.bindings[0].clone());
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn invalid_module_evidence_blocks_the_binding() {
        let mut request = fixture_request();
        request.modules[0].claim_ceiling = "a more ambitious claim".to_owned();
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    fn pressure_fixture_request() -> KitchenCapabilityAdmissionRequestV1 {
        let mut request = fixture_request();
        let module_bundle: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/fixtures/kitchen-module-capability-v1.json"
        ))
        .expect("module fixture bundle must parse");
        let pressure_module: KitchenModuleCapabilityV1 =
            serde_json::from_value(module_bundle["cases"][1]["module"].clone())
                .expect("independently qualified pressure fixture must deserialize");

        request.modules = vec![pressure_module];
        request.recipe.tasks[0].action_kind = RecipeActionKindV1::PressureCook;
        request.recipe.tasks[0].ontology_binding = Some(rule_based_action_binding(
            RecipeActionKindV1::PressureCook,
            RecipeEvidenceClassV1::Declared,
        ));
        request.bindings[0] = KitchenTaskModuleBindingV1 {
            task_id: "chop-onion".to_owned(),
            module_id: "fixture:pressure-cooker-01".to_owned(),
            module_revision: "pressure-rev-a".to_owned(),
            capability_id: "capability:pressure-cook".to_owned(),
            input_form_ref: "form:ingredients-in-vessel".to_owned(),
            output_form_ref: "form:cooked-meal".to_owned(),
            scheduled_resource_id: "processor".to_owned(),
        };
        request.request_id = "fixture:pressure-cook-admission".to_owned();
        request
    }

    #[test]
    fn pressure_module_requires_independent_vessel_and_relief_qualification() {
        let request = pressure_fixture_request();
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::WithinDeclaredEnvelope
        );

        let mut request = pressure_fixture_request();
        request.modules[0].pressure_safety = None;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn module_peak_and_contiguous_surge_limits_are_enforced() {
        let mut request = fixture_request();
        request.power_plan.task_profiles[0].segments[0].real_power_w = 1_300;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );

        let mut request = fixture_request();
        request.power_plan.task_profiles[0].segments[0].real_power_w = 1_100;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn missing_module_electrical_limits_remain_indeterminate() {
        let mut request = fixture_request();
        request.modules[0].electrical.continuous_real_power_limit_w = None;
        request.modules[0].electrical.peak_real_power_limit_w = None;
        request.modules[0].electrical.maximum_surge_duration_ms = None;
        request.modules[0].electrical.power_profile_ref = None;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Indeterminate
        );
    }

    #[test]
    fn household_power_schedule_is_revalidated_and_bound_to_recipe() {
        let mut request = fixture_request();
        request.power_plan.envelope.continuous_real_power_limit_w = 300;
        request.power_plan.envelope.surge_real_power_limit_w = 400;
        request.power_plan.envelope.max_surge_duration_ms = 1;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );

        let mut request = fixture_request();
        request.power_plan.plan.tasks[0].duration_ms += 1;
        assert_eq!(
            disposition(&request),
            KitchenCapabilityDispositionV1::Blocked
        );
    }

    #[test]
    fn admission_case_manifest_is_versioned_and_synthetic() {
        #[derive(Deserialize)]
        struct FixtureBundle {
            fixture_bundle_id: String,
            status: String,
            cases: Vec<FixtureCase>,
        }
        #[derive(Deserialize)]
        struct FixtureCase {
            case_id: String,
            expected_disposition: String,
        }
        let bundle: FixtureBundle = serde_json::from_str(include_str!(
            "../schemas/fixtures/kitchen-capability-admission-v1.json"
        ))
        .expect("admission fixture manifest must parse");
        assert_eq!(bundle.fixture_bundle_id, "kitchen-capability-admission-v1");
        assert_eq!(bundle.status, "synthetic_only_not_physical_qualification");
        assert!(bundle.cases.len() >= 10);
        assert!(bundle.cases.iter().all(|case| {
            !case.case_id.trim().is_empty()
                && matches!(
                    case.expected_disposition.as_str(),
                    "within_declared_envelope" | "indeterminate" | "blocked"
                )
        }));
    }
}

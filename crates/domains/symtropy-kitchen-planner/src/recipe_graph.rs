// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-aware recipe action graphs that map to the existing kitchen planner.
//!
//! This is a typed task representation, not a recipe generator, safety
//! assessor, allergen validator or appliance controller. It stores references
//! to recipe instructions rather than copying source text. Mapping preserves
//! the embedded KitchenTaskV1 scheduling fields exactly.

use crate::{KitchenPlanV1, KitchenTaskV1, KITCHEN_PLAN_SCHEMA_VERSION, validate_plan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const RECIPE_TASK_GRAPH_SCHEMA_VERSION: u32 = 1;
pub const RECIPE_GRAPH_CLAIM_CEILING: &str =
    "Recipe task graph only; does not validate food safety, allergens, ingredient condition, or culinary quality.";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeRightsStatusV1 {
    PublicDomain,
    Licensed,
    UserSupplied,
    PermissionGranted,
    Unknown,
    Restricted,
}

#[derive(
    Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum RecipeUseScopeV1 {
    LocalProcessing,
    Redistribution,
    ModelTraining,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipeSourceV1 {
    /// Stable source pointer. It may refer to a user-supplied local document.
    pub source_ref: String,
    pub rights_status: RecipeRightsStatusV1,
    /// Explicitly permitted uses. Unknown/restricted rights are local-only.
    pub permitted_uses: Vec<RecipeUseScopeV1>,
    /// Required to share or use the derived graph for model training.
    pub permission_evidence_ref: Option<String>,
    /// Optional SHA-256 of the source artifact; this authenticates bytes only.
    pub content_sha256: Option<String>,
    /// Optional attribution pointer; attribution alone is not permission.
    pub attribution_ref: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeEvidenceClassV1 {
    Declared,
    Modelled,
    Simulated,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeReviewStateV1 {
    Unreviewed,
    Reviewed,
    Rejected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeQuantityUnitV1 {
    Milligram,
    Gram,
    Kilogram,
    Millilitre,
    Litre,
    Piece,
    Count,
    Teaspoon,
    Tablespoon,
    Cup,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExactRecipeQuantityV1 {
    /// Unsigned integer coefficient; decimal_places supplies the decimal scale.
    /// Example: value=125, decimal_places=2, unit=gram means 1.25 g.
    pub value: u64,
    pub decimal_places: u8,
    pub unit: RecipeQuantityUnitV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipeIngredientV1 {
    pub ingredient_id: String,
    pub display_name: String,
    /// Optional canonical Mycelix lot reference; never infer a lot from a name.
    pub product_lot_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipeIngredientUseV1 {
    pub ingredient_id: String,
    pub quantity: ExactRecipeQuantityV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeActionKindV1 {
    Wash,
    Peel,
    Trim,
    Chop,
    Slice,
    Dice,
    Grate,
    Shred,
    Mix,
    Knead,
    Blend,
    Emulsify,
    Saute,
    Boil,
    Simmer,
    Steam,
    Bake,
    Roast,
    PressureCook,
    Cool,
    Rest,
    Serve,
    Other,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipeTaskNodeV1 {
    /// The existing planner task is the scheduling authority. Its fields are
    /// copied unchanged by the mapper.
    pub schedule: KitchenTaskV1,
    pub action_kind: RecipeActionKindV1,
    /// Required for action_kind=Other; may optionally clarify other actions.
    pub action_label: Option<String>,
    pub ingredient_inputs: Vec<RecipeIngredientUseV1>,
    pub output_refs: Vec<String>,
    /// Reference only; raw source instructions are not embedded or recopied.
    pub instruction_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipeTaskGraphV1 {
    pub schema_version: u32,
    pub recipe_id: String,
    pub revision: u32,
    pub title: String,
    pub source: RecipeSourceV1,
    pub evidence_class: RecipeEvidenceClassV1,
    pub review_state: RecipeReviewStateV1,
    pub ingredients: Vec<RecipeIngredientV1>,
    pub tasks: Vec<RecipeTaskNodeV1>,
    pub claim_ceiling: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeGraphViolationCodeV1 {
    UnsupportedSchemaVersion,
    MissingRequiredValue,
    InvalidRevision,
    InvalidSourceDigest,
    InsufficientRightsEvidence,
    DuplicateUseScope,
    DuplicateIngredientId,
    DuplicateTaskId,
    UnknownIngredientReference,
    DuplicateIngredientUse,
    InvalidQuantity,
    InvalidQuantityScale,
    MissingOtherActionLabel,
    InvalidOutputReference,
    RejectedByHumanReview,
    ClaimCeilingMismatch,
    InvalidSchedulePlan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipeGraphViolationV1 {
    pub code: RecipeGraphViolationCodeV1,
    pub recipe_id: Option<String>,
    pub task_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RecipeGraphValidationReportV1 {
    pub recipe_id: String,
    pub violations: Vec<RecipeGraphViolationV1>,
}

impl RecipeGraphValidationReportV1 {
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }
}

fn add_violation(
    violations: &mut Vec<RecipeGraphViolationV1>,
    code: RecipeGraphViolationCodeV1,
    recipe_id: Option<&str>,
    task_id: Option<&str>,
    detail: impl Into<String>,
) {
    violations.push(RecipeGraphViolationV1 {
        code,
        recipe_id: recipe_id.map(str::to_owned),
        task_id: task_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Validate the recipe graph and its embedded schedule plan without changing
/// task order, units, dependencies or ingredient quantities.
pub fn validate_recipe_task_graph(
    graph: &RecipeTaskGraphV1,
) -> RecipeGraphValidationReportV1 {
    let mut violations = Vec::new();
    let recipe_id = graph.recipe_id.as_str();

    if graph.schema_version != RECIPE_TASK_GRAPH_SCHEMA_VERSION {
        add_violation(
            &mut violations,
            RecipeGraphViolationCodeV1::UnsupportedSchemaVersion,
            Some(recipe_id),
            None,
            format!("unsupported recipe task graph schema version {}", graph.schema_version),
        );
    }
    if !nonempty(recipe_id) || !nonempty(&graph.title) || !nonempty(&graph.source.source_ref) {
        add_violation(
            &mut violations,
            RecipeGraphViolationCodeV1::MissingRequiredValue,
            Some(recipe_id),
            None,
            "recipe_id, title and source_ref must be non-empty",
        );
    }
    if graph.revision == 0 {
        add_violation(
            &mut violations,
            RecipeGraphViolationCodeV1::InvalidRevision,
            Some(recipe_id),
            None,
            "recipe revision must be positive",
        );
    }
    if let Some(digest) = graph.source.content_sha256.as_deref()
        && (digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        add_violation(
            &mut violations,
            RecipeGraphViolationCodeV1::InvalidSourceDigest,
            Some(recipe_id),
            None,
            "content_sha256 must contain exactly 64 hexadecimal characters",
        );
    }
    if graph.source.permitted_uses.is_empty() {
        add_violation(
            &mut violations,
            RecipeGraphViolationCodeV1::InsufficientRightsEvidence,
            Some(recipe_id),
            None,
            "at least one explicit permitted use is required",
        );
    }
    let mut use_scopes = BTreeSet::new();
    for scope in &graph.source.permitted_uses {
        if !use_scopes.insert(*scope) {
            add_violation(
                &mut violations,
                RecipeGraphViolationCodeV1::DuplicateUseScope,
                Some(recipe_id),
                None,
                "permitted_uses must not contain duplicate scopes",
            );
        }
    }
    let has_non_local_use = graph.source.permitted_uses.iter().any(|scope| {
        *scope != RecipeUseScopeV1::LocalProcessing
    });
    if has_non_local_use
        && (graph.source.permission_evidence_ref.as_deref().is_none_or(|value| !nonempty(value))
            || matches!(
                graph.source.rights_status,
                RecipeRightsStatusV1::Unknown | RecipeRightsStatusV1::Restricted
            ))
    {
        add_violation(
            &mut violations,
            RecipeGraphViolationCodeV1::InsufficientRightsEvidence,
            Some(recipe_id),
            None,
            "redistribution/model-training scope requires explicit permission evidence and non-restricted source rights",
        );
    }
    if graph.source.permission_evidence_ref.as_deref().is_some_and(|value| !nonempty(value)) {
        add_violation(
            &mut violations,
            RecipeGraphViolationCodeV1::InsufficientRightsEvidence,
            Some(recipe_id),
            None,
            "permission_evidence_ref must be non-empty when supplied",
        );
    }

    if graph.claim_ceiling != RECIPE_GRAPH_CLAIM_CEILING {
        add_violation(
            &mut violations,
            RecipeGraphViolationCodeV1::ClaimCeilingMismatch,
            Some(recipe_id),
            None,
            "recipe graph claim ceiling must match the fixed non-safety claim",
        );
    }
    if graph.review_state == RecipeReviewStateV1::Rejected {
        add_violation(
            &mut violations,
            RecipeGraphViolationCodeV1::RejectedByHumanReview,
            Some(recipe_id),
            None,
            "a graph rejected by human review cannot be mapped to a schedule plan",
        );
    }

    let mut ingredient_ids = BTreeSet::new();
    for ingredient in &graph.ingredients {
        if !nonempty(&ingredient.ingredient_id) || !nonempty(&ingredient.display_name) {
            add_violation(
                &mut violations,
                RecipeGraphViolationCodeV1::MissingRequiredValue,
                Some(recipe_id),
                None,
                "ingredient_id and display_name must be non-empty",
            );
        }
        if !ingredient_ids.insert(ingredient.ingredient_id.as_str()) {
            add_violation(
                &mut violations,
                RecipeGraphViolationCodeV1::DuplicateIngredientId,
                Some(recipe_id),
                None,
                format!("duplicate ingredient_id {}", ingredient.ingredient_id),
            );
        }
        if ingredient.product_lot_ref.as_deref().is_some_and(|value| !nonempty(value)) {
            add_violation(
                &mut violations,
                RecipeGraphViolationCodeV1::MissingRequiredValue,
                Some(recipe_id),
                None,
                "product_lot_ref must be non-empty when supplied",
            );
        }
    }

    let schedule_plan = KitchenPlanV1 {
        schema_version: KITCHEN_PLAN_SCHEMA_VERSION,
        plan_id: graph.recipe_id.clone(),
        tasks: graph.tasks.iter().map(|node| node.schedule.clone()).collect(),
    };
    let schedule_report = validate_plan(&schedule_plan);
    if !schedule_report.is_valid() {
        add_violation(
            &mut violations,
            RecipeGraphViolationCodeV1::InvalidSchedulePlan,
            Some(recipe_id),
            None,
            format!("embedded KitchenPlanV1 has {} validation violation(s)", schedule_report.violations.len()),
        );
    }

    let mut task_ids = BTreeSet::new();
    for node in &graph.tasks {
        let task_id = node.schedule.task_id.as_str();
        if !task_ids.insert(task_id) {
            add_violation(
                &mut violations,
                RecipeGraphViolationCodeV1::DuplicateTaskId,
                Some(recipe_id),
                Some(task_id),
                format!("duplicate recipe task id {task_id}"),
            );
        }
        if node.action_kind == RecipeActionKindV1::Other
            && node.action_label.as_deref().is_none_or(|label| !nonempty(label))
        {
            add_violation(
                &mut violations,
                RecipeGraphViolationCodeV1::MissingOtherActionLabel,
                Some(recipe_id),
                Some(task_id),
                "action_label is required when action_kind is other",
            );
        }
        if node.instruction_ref.as_deref().is_some_and(|value| !nonempty(value)) {
            add_violation(
                &mut violations,
                RecipeGraphViolationCodeV1::MissingRequiredValue,
                Some(recipe_id),
                Some(task_id),
                "instruction_ref must be non-empty when supplied",
            );
        }

        let mut ingredient_uses = BTreeSet::new();
        for usage in &node.ingredient_inputs {
            if !ingredient_ids.contains(usage.ingredient_id.as_str()) {
                add_violation(
                    &mut violations,
                    RecipeGraphViolationCodeV1::UnknownIngredientReference,
                    Some(recipe_id),
                    Some(task_id),
                    format!("ingredient reference {} is not declared", usage.ingredient_id),
                );
            }
            if !ingredient_uses.insert(usage.ingredient_id.as_str()) {
                add_violation(
                    &mut violations,
                    RecipeGraphViolationCodeV1::DuplicateIngredientUse,
                    Some(recipe_id),
                    Some(task_id),
                    format!("ingredient {} is listed more than once; combine quantities explicitly", usage.ingredient_id),
                );
            }
            if usage.quantity.value == 0 {
                add_violation(
                    &mut violations,
                    RecipeGraphViolationCodeV1::InvalidQuantity,
                    Some(recipe_id),
                    Some(task_id),
                    format!("ingredient {} quantity must be positive", usage.ingredient_id),
                );
            }
            if usage.quantity.decimal_places > 6 {
                add_violation(
                    &mut violations,
                    RecipeGraphViolationCodeV1::InvalidQuantityScale,
                    Some(recipe_id),
                    Some(task_id),
                    format!("ingredient {} quantity scale exceeds six decimal places", usage.ingredient_id),
                );
            }
        }
        let mut output_refs = BTreeSet::new();
        for output_ref in &node.output_refs {
            if !nonempty(output_ref) || !output_refs.insert(output_ref.as_str()) {
                add_violation(
                    &mut violations,
                    RecipeGraphViolationCodeV1::InvalidOutputReference,
                    Some(recipe_id),
                    Some(task_id),
                    "output refs must be non-empty and unique per task",
                );
            }
        }
    }

    violations.sort_by(|a, b| {
        (a.code, a.task_id.as_deref().unwrap_or(""), a.detail.as_str()).cmp(&(
            b.code,
            b.task_id.as_deref().unwrap_or(""),
            b.detail.as_str(),
        ))
    });

    RecipeGraphValidationReportV1 {
        recipe_id: graph.recipe_id.clone(),
        violations,
    }
}

/// Map to the existing scheduler plan. No timing, dependency, resource or task
/// ID fields are synthesized or modified. The result remains advisory and has
/// no appliance actuation authority.
pub fn recipe_graph_to_kitchen_plan(
    graph: &RecipeTaskGraphV1,
) -> Result<KitchenPlanV1, RecipeGraphValidationReportV1> {
    let report = validate_recipe_task_graph(graph);
    if !report.is_valid() {
        return Err(report);
    }
    Ok(KitchenPlanV1 {
        schema_version: KITCHEN_PLAN_SCHEMA_VERSION,
        plan_id: graph.recipe_id.clone(),
        tasks: graph.tasks.iter().map(|node| node.schedule.clone()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TaskDependencyV1;

    fn valid_graph() -> RecipeTaskGraphV1 {
        RecipeTaskGraphV1 {
            schema_version: RECIPE_TASK_GRAPH_SCHEMA_VERSION,
            recipe_id: "fixture:onion-sauce".to_owned(),
            revision: 1,
            title: "Synthetic onion sauce".to_owned(),
            source: RecipeSourceV1 {
                source_ref: "fixture:synthetic-recipe".to_owned(),
                rights_status: RecipeRightsStatusV1::UserSupplied,
                permitted_uses: vec![RecipeUseScopeV1::LocalProcessing],
                permission_evidence_ref: None,
                content_sha256: None,
                attribution_ref: None,
            },
            evidence_class: RecipeEvidenceClassV1::Declared,
            review_state: RecipeReviewStateV1::Reviewed,
            ingredients: vec![RecipeIngredientV1 {
                ingredient_id: "onion".to_owned(),
                display_name: "Onion".to_owned(),
                product_lot_ref: None,
            }],
            tasks: vec![
                RecipeTaskNodeV1 {
                    schedule: KitchenTaskV1 {
                        task_id: "chop-onion".to_owned(),
                        duration_ms: 30_000,
                        resource_ids: vec!["processor".to_owned()],
                        dependencies: vec![],
                        earliest_start_ms: Some(0),
                        latest_finish_ms: Some(60_000),
                    },
                    action_kind: RecipeActionKindV1::Chop,
                    action_label: None,
                    ingredient_inputs: vec![RecipeIngredientUseV1 {
                        ingredient_id: "onion".to_owned(),
                        quantity: ExactRecipeQuantityV1 {
                            value: 250,
                            decimal_places: 0,
                            unit: RecipeQuantityUnitV1::Gram,
                        },
                    }],
                    output_refs: vec!["prepared-onion".to_owned()],
                    instruction_ref: None,
                },
                RecipeTaskNodeV1 {
                    schedule: KitchenTaskV1 {
                        task_id: "saute-onion".to_owned(),
                        duration_ms: 180_000,
                        resource_ids: vec!["burner-1".to_owned()],
                        dependencies: vec![TaskDependencyV1 {
                            predecessor_task_id: "chop-onion".to_owned(),
                            min_lag_ms: 0,
                            max_lag_ms: Some(30_000),
                        }],
                        earliest_start_ms: None,
                        latest_finish_ms: None,
                    },
                    action_kind: RecipeActionKindV1::Saute,
                    action_label: Some("Sweat until translucent".to_owned()),
                    ingredient_inputs: vec![RecipeIngredientUseV1 {
                        ingredient_id: "onion".to_owned(),
                        quantity: ExactRecipeQuantityV1 {
                            value: 250,
                            decimal_places: 0,
                            unit: RecipeQuantityUnitV1::Gram,
                        },
                    }],
                    output_refs: vec!["sweated-onion".to_owned()],
                    instruction_ref: Some("instruction-ref:synthetic-step-2".to_owned()),
                },
            ],
            claim_ceiling: RECIPE_GRAPH_CLAIM_CEILING.to_owned(),
        }
    }

    #[test]
    fn maps_recipe_graph_without_changing_scheduler_fields() {
        let graph = valid_graph();
        let mapped = recipe_graph_to_kitchen_plan(&graph).unwrap();
        assert_eq!(mapped.plan_id, graph.recipe_id);
        assert_eq!(
            mapped.tasks,
            graph.tasks.iter().map(|node| node.schedule.clone()).collect::<Vec<_>>()
        );
        assert!(validate_plan(&mapped).is_valid());
    }

    #[test]
    fn unknown_ingredient_reference_is_rejected() {
        let mut graph = valid_graph();
        graph.tasks[0].ingredient_inputs[0].ingredient_id = "missing".to_owned();
        let report = validate_recipe_task_graph(&graph);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecipeGraphViolationCodeV1::UnknownIngredientReference
        }));
        assert!(recipe_graph_to_kitchen_plan(&graph).is_err());
    }

    #[test]
    fn duplicate_ingredient_ids_are_rejected() {
        let mut graph = valid_graph();
        graph.ingredients.push(graph.ingredients[0].clone());
        let report = validate_recipe_task_graph(&graph);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecipeGraphViolationCodeV1::DuplicateIngredientId
        }));
    }

    #[test]
    fn unlabelled_other_action_is_rejected() {
        let mut graph = valid_graph();
        graph.tasks[0].action_kind = RecipeActionKindV1::Other;
        graph.tasks[0].action_label = None;
        let report = validate_recipe_task_graph(&graph);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecipeGraphViolationCodeV1::MissingOtherActionLabel
        }));
    }

    #[test]
    fn rejected_human_review_cannot_be_mapped() {
        let mut graph = valid_graph();
        graph.review_state = RecipeReviewStateV1::Rejected;
        let report = validate_recipe_task_graph(&graph);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecipeGraphViolationCodeV1::RejectedByHumanReview
        }));
    }

    #[test]
    fn unknown_source_rights_cannot_be_used_for_model_training() {
        let mut graph = valid_graph();
        graph.source.rights_status = RecipeRightsStatusV1::Unknown;
        graph.evidence_class = RecipeEvidenceClassV1::Modelled;
        graph.source.permitted_uses = vec![
            RecipeUseScopeV1::LocalProcessing,
            RecipeUseScopeV1::ModelTraining,
        ];
        let report = validate_recipe_task_graph(&graph);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecipeGraphViolationCodeV1::InsufficientRightsEvidence
        }));
        assert_eq!(graph.source.rights_status, RecipeRightsStatusV1::Unknown);
        assert_eq!(graph.evidence_class, RecipeEvidenceClassV1::Modelled);
    }

    #[test]
    fn duplicate_permitted_use_scopes_are_rejected() {
        let mut graph = valid_graph();
        graph.source.permitted_uses = vec![
            RecipeUseScopeV1::LocalProcessing,
            RecipeUseScopeV1::LocalProcessing,
        ];
        let report = validate_recipe_task_graph(&graph);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecipeGraphViolationCodeV1::DuplicateUseScope
        }));
    }

    #[test]
    fn invalid_source_digest_is_rejected() {
        let mut graph = valid_graph();
        graph.source.content_sha256 = Some("not-a-sha256-digest".to_owned());
        let report = validate_recipe_task_graph(&graph);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecipeGraphViolationCodeV1::InvalidSourceDigest
        }));
    }

    #[test]
    fn zero_quantity_and_excessive_scale_are_rejected() {
        let mut graph = valid_graph();
        graph.tasks[0].ingredient_inputs[0].quantity.value = 0;
        graph.tasks[0].ingredient_inputs[0].quantity.decimal_places = 7;
        let report = validate_recipe_task_graph(&graph);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecipeGraphViolationCodeV1::InvalidQuantity
        }));
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecipeGraphViolationCodeV1::InvalidQuantityScale
        }));
    }

    #[test]
    fn local_processing_with_unknown_source_rights_is_explicitly_local_only() {
        let mut graph = valid_graph();
        graph.source.rights_status = RecipeRightsStatusV1::Unknown;
        graph.source.permitted_uses = vec![RecipeUseScopeV1::LocalProcessing];
        assert!(validate_recipe_task_graph(&graph).is_valid());
    }

    #[test]
    fn model_training_scope_requires_explicit_permission_evidence() {
        let mut graph = valid_graph();
        graph.source.permitted_uses = vec![
            RecipeUseScopeV1::LocalProcessing,
            RecipeUseScopeV1::ModelTraining,
        ];
        graph.source.permission_evidence_ref = Some("permission:fixture-grant".to_owned());
        let report = validate_recipe_task_graph(&graph);
        assert!(report.is_valid());
    }

    #[test]
    fn claim_ceiling_must_not_be_weakened() {
        let mut graph = valid_graph();
        graph.claim_ceiling = "safe to eat".to_owned();
        let report = validate_recipe_task_graph(&graph);
        assert!(report.violations.iter().any(|violation| {
            violation.code == RecipeGraphViolationCodeV1::ClaimCeilingMismatch
        }));
    }
}

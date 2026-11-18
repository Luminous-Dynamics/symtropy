// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evidence-bounded compiler from a Symthaea MealProposalV1 into two linked
//! representations: a recipe action graph and a typed food-material flow graph.
//!
//! The compiler accepts explicit, reviewable resolutions. It never infers
//! quantities, actions, durations, planning resources, household stock or
//! appliance modules. Planned material flows are not measurements or proof of
//! edible acceptance. The result is not a safety certificate or actuation permit.

use crate::action_ontology::{ActionMappingMethodV1, ActionOntologyBindingV1};
use crate::recipe_graph::{
    ExactRecipeQuantityV1, RecipeActionKindV1, RecipeEvidenceClassV1, RecipeGraphValidationReportV1,
    RecipeIngredientUseV1, RecipeIngredientV1, RecipeRightsStatusV1, RecipeReviewStateV1,
    RecipeSourceV1, RecipeTaskGraphV1, RecipeTaskNodeV1, RecipeUseScopeV1,
    RECIPE_GRAPH_CLAIM_CEILING, RECIPE_TASK_GRAPH_SCHEMA_VERSION, validate_recipe_task_graph,
};
use crate::KitchenTaskV1;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MEAL_COMPILATION_REQUEST_SCHEMA_VERSION: u32 = 1;
pub const MEAL_COMPILATION_REPORT_SCHEMA_VERSION: u32 = 1;
pub const MEAL_COMPILER_VERSION_REF: &str = "symtropy-kitchen-meal-compiler-v1";
pub const MEAL_PROPOSAL_CLAIM_CEILING: &str = "A user-reviewable culinary proposal with scoped source references and explicit uncertainty; not verified household inventory, nutrition/health advice, food-safety approval, equipment compatibility, a schedule, or actuation authority.";
pub const MEAL_COMPILATION_CLAIM_CEILING: &str = "Deterministic compilation of a supplied, accepted meal proposal and explicit reviewed task/material-flow resolutions only; not proof of household inventory, freshness, food safety, allergen status, physical compatibility, measured process yield, sensory acceptance, appliance safety, schedule feasibility, or authority to actuate.";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityDimensionV1 {
    Mass,
    Volume,
    Count,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityUnitV1 {
    Milligram,
    Gram,
    Kilogram,
    Millilitre,
    Litre,
    Piece,
    Count,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExactIngredientQuantityV1 {
    pub dimension: QuantityDimensionV1,
    pub value: u64,
    pub decimal_places: u8,
    pub unit: QuantityUnitV1,
}

impl ExactIngredientQuantityV1 {
    pub fn is_valid(&self) -> bool {
        if self.decimal_places > 6 || self.value == 0 {
            return false;
        }
        matches!(
            (self.dimension, self.unit),
            (QuantityDimensionV1::Mass, QuantityUnitV1::Milligram)
                | (QuantityDimensionV1::Mass, QuantityUnitV1::Gram)
                | (QuantityDimensionV1::Mass, QuantityUnitV1::Kilogram)
                | (QuantityDimensionV1::Volume, QuantityUnitV1::Millilitre)
                | (QuantityDimensionV1::Volume, QuantityUnitV1::Litre)
                | (QuantityDimensionV1::Count, QuantityUnitV1::Piece)
                | (QuantityDimensionV1::Count, QuantityUnitV1::Count)
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalEvidenceClassV1 {
    UserSpecified,
    Observed,
    Measured,
    Published,
    VendorDeclared,
    Modelled,
    Simulated,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactScopeV1 {
    GenericIngredient,
    ProductSku,
    SupplierLot,
    RecipeProcess,
    MarketBasket,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CulinaryFactKindV1 {
    Price,
    Nutrient,
    IngredientIdentity,
    Package,
    AllergenDeclaration,
    Availability,
    Quality,
    ProcessProperty,
    Other,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CulinaryFactV1 {
    pub fact_id: String,
    pub ingredient_id: Option<String>,
    pub kind: CulinaryFactKindV1,
    pub scope: FactScopeV1,
    pub value: serde_json::Value,
    pub unit: Option<String>,
    pub basis: Option<String>,
    pub source_ref: String,
    pub source_record_id: String,
    pub source_version: String,
    pub evidence_class: ProposalEvidenceClassV1,
    pub observed_at_epoch_ms: u64,
    pub valid_until_epoch_ms: Option<u64>,
    pub market_ref: Option<String>,
    pub license_ref: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IngredientQuantityStateV1 {
    Explicit,
    Unknown,
    NotRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IngredientFoodStateV1 {
    Raw,
    Cooked,
    Dried,
    Frozen,
    Drained,
    Rehydrated,
    Fermented,
    Other,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalIngredientV1 {
    pub ingredient_id: String,
    pub display_name: String,
    pub quantity_state: IngredientQuantityStateV1,
    pub quantity: Option<ExactIngredientQuantityV1>,
    pub food_state: IngredientFoodStateV1,
    pub product_lot_ref: Option<String>,
    pub fact_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedTaskV1 {
    pub task_id: String,
    pub description: String,
    pub action_label: Option<String>,
    pub ingredient_refs: Vec<String>,
    pub depends_on: Vec<String>,
    pub estimated_duration_min_ms: Option<u64>,
    pub estimated_duration_max_ms: Option<u64>,
    pub fact_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UserMealConstraintsV1 {
    pub servings: u32,
    pub time_budget_ms: Option<u64>,
    pub budget_minor_units: Option<u64>,
    pub currency_code: Option<String>,
    pub preferred_cuisine_tags: Vec<String>,
    pub user_declared_exclusions: Vec<String>,
    pub user_declared_preferences: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalReviewStateV1 {
    Proposed,
    HumanReviewed,
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyImpactV1 {
    PlanningBlocker,
    CandidateScoringOnly,
    Informational,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalUncertaintyV1 {
    pub field_path: String,
    pub reason: String,
    pub impact: UncertaintyImpactV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CulinaryProposalSourceV1 {
    pub source_ref: String,
    pub content_digest_sha256: Option<String>,
    pub license_ref: Option<String>,
    pub attribution: Option<String>,
    pub evidence_class: ProposalEvidenceClassV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MealProposalV1 {
    pub schema_version: u32,
    pub proposal_id: String,
    pub revision: u32,
    pub title: String,
    pub source: CulinaryProposalSourceV1,
    pub constraints: UserMealConstraintsV1,
    pub ingredients: Vec<ProposalIngredientV1>,
    pub tasks: Vec<ProposedTaskV1>,
    pub facts: Vec<CulinaryFactV1>,
    pub uncertainties: Vec<ProposalUncertaintyV1>,
    pub review_state: ProposalReviewStateV1,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalAcceptanceV1 {
    /// Exact proposal digest accepted by the household reviewer.
    pub proposal_sha256: String,
    pub proposal_revision: u32,
    pub accepted_by_ref: String,
    pub accepted_at_epoch_ms: u64,
    pub acceptance_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MealSourceResolutionV1 {
    pub source_ref: String,
    pub rights_status: RecipeRightsStatusV1,
    pub permitted_uses: Vec<RecipeUseScopeV1>,
    pub permission_evidence_ref: Option<String>,
    pub reviewed_resolution_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannedTaskOutputV1 {
    pub output_ref: String,
    pub quantity: ExactIngredientQuantityV1,
    /// Original proposal ingredient IDs represented by this output.
    pub ingredient_origins: Vec<String>,
    /// Reference to the declared/modelled planning basis, not a measurement.
    pub quantity_basis_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MealTaskCompilationBindingV1 {
    pub task_id: String,
    pub action_kind: RecipeActionKindV1,
    pub action_label: Option<String>,
    /// Exact scheduler input; resource IDs are planning resources, not modules.
    pub schedule: KitchenTaskV1,
    /// Required explicit ontology binding; no lexical inference is performed.
    pub ontology_binding: Option<ActionOntologyBindingV1>,
    pub action_resolution_ref: String,
    pub duration_method_ref: String,
    pub schedule_resolution_ref: String,
    pub resource_resolution_ref: String,
    pub instruction_ref: Option<String>,
    pub outputs: Vec<PlannedTaskOutputV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FoodFlowSourceV1 {
    Ingredient { ingredient_id: String },
    TaskOutput { task_id: String, output_ref: String },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedLossKindV1 {
    Trim,
    ExpectedReject,
    ExpectedResidue,
    ExpectedDrainage,
    ExpectedEvaporation,
    Other,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FoodFlowDestinationV1 {
    TaskInput { task_id: String },
    PlannedEdibleOutput { output_ref: String },
    PlannedLossOutput {
        output_ref: String,
        loss_kind: PlannedLossKindV1,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FoodFlowEdgeV1 {
    pub edge_id: String,
    pub material_ref: String,
    pub source: FoodFlowSourceV1,
    pub destination: FoodFlowDestinationV1,
    pub quantity: ExactIngredientQuantityV1,
    /// Reference to the source, reviewed allocation or planning basis.
    pub evidence_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FoodFlowGraphV1 {
    pub schema_version: u32,
    pub proposal_id: String,
    pub proposal_revision: u32,
    pub edges: Vec<FoodFlowEdgeV1>,
    pub claim_ceiling: String,
}

pub const FOOD_FLOW_GRAPH_CLAIM_CEILING: &str = "Planned typed ingredient and intermediate material allocations only; not measured process yield, mass-balance closure, physical inventory, food safety, sensory acceptance, or disposal evidence.";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MealCompilationRequestV1 {
    pub schema_version: u32,
    pub request_id: String,
    pub compiled_at_epoch_ms: u64,
    /// SHA-256 over serde_json serialization of the typed MealProposalV1.
    pub proposal_sha256: String,
    pub proposal: MealProposalV1,
    pub acceptance: ProposalAcceptanceV1,
    pub source_resolution: MealSourceResolutionV1,
    pub task_bindings: Vec<MealTaskCompilationBindingV1>,
    pub food_flow_edges: Vec<FoodFlowEdgeV1>,
    pub claim_ceiling: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MealCompilationDispositionV1 {
    Compiled,
    Indeterminate,
    Blocked,
    Conflicted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MealCompilationIssueCodeV1 {
    UnsupportedSchemaVersion,
    MissingRequiredValue,
    InvalidClaimCeiling,
    InvalidProposalDigest,
    InvalidProposal,
    DuplicateIngredientId,
    DuplicateTaskId,
    DuplicateFactId,
    UnknownIngredientReference,
    UnknownFactReference,
    UnknownFactIngredient,
    InvalidQuantity,
    QuantityStateMismatch,
    UnknownQuantity,
    InvalidFactTimeWindow,
    StaleFact,
    FactIngredientMismatch,
    InvalidAllergenFactScope,
    DuplicateUncertainty,
    PlanningBlockerUnresolved,
    DependencyCycle,
    ProposalNotAccepted,
    ProposalRejected,
    SourceResolutionMismatch,
    MissingSourceResolution,
    InsufficientSourceRights,
    DuplicateTaskBinding,
    MissingTaskBinding,
    UnknownTaskBinding,
    TaskIdMismatch,
    DependencyBindingMismatch,
    DurationOutsideProposalWindow,
    ProposalAcceptanceMismatch,
    ProposalAcceptanceMissing,
    MissingActionResolution,
    MissingDurationEvidence,
    MissingResourceResolution,
    MissingPlanningResource,
    MissingActionOntology,
    MissingTaskOutput,
    DuplicateTaskOutput,
    InvalidTaskOutput,
    MissingTaskOutputFlow,
    DuplicateFlowId,
    DuplicateMaterialRef,
    DuplicateTerminalOutputRef,
    IngredientAllocationIncomplete,
    UnknownFlowIngredient,
    UnknownFlowTask,
    UnknownFlowOutput,
    FlowEndpointMismatch,
    FlowQuantityMismatch,
    InputQuantityExceedsProposal,
    InputQuantityArithmeticOverflow,
    MissingTaskIngredientFlow,
    DuplicateTaskIngredientFlow,
    TaskIngredientLineageMismatch,
    OutputIngredientLineageMismatch,
    TaskOutputLineageIncomplete,
    InvalidCompiledRecipeGraph,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MealCompilationIssueV1 {
    pub code: MealCompilationIssueCodeV1,
    pub reference: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MealCompilationReportV1 {
    pub schema_version: u32,
    pub request_id: String,
    pub proposal_id: String,
    pub proposal_revision: u32,
    pub proposal_sha256: String,
    pub compiler_version_ref: String,
    pub disposition: MealCompilationDispositionV1,
    pub issues: Vec<MealCompilationIssueV1>,
    pub resolution_refs: Vec<String>,
    pub recipe_graph: Option<RecipeTaskGraphV1>,
    pub food_flow_graph: Option<FoodFlowGraphV1>,
    pub claim_ceiling: String,
}

impl MealCompilationReportV1 {
    pub fn is_compiled(&self) -> bool {
        self.disposition == MealCompilationDispositionV1::Compiled
            && self.recipe_graph.is_some()
            && self.food_flow_graph.is_some()
            && self.issues.is_empty()
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn add_issue(
    issues: &mut Vec<MealCompilationIssueV1>,
    code: MealCompilationIssueCodeV1,
    reference: Option<&str>,
    detail: impl Into<String>,
) {
    issues.push(MealCompilationIssueV1 {
        code,
        reference: reference.map(str::to_owned),
        detail: detail.into(),
    });
}

fn sort_issues(issues: &mut [MealCompilationIssueV1]) {
    issues.sort_by(|left, right| {
        (left.code, left.reference.as_deref().unwrap_or(""), left.detail.as_str()).cmp(&(
            right.code,
            right.reference.as_deref().unwrap_or(""),
            right.detail.as_str(),
        ))
    });
}

fn disposition(issues: &[MealCompilationIssueV1]) -> MealCompilationDispositionV1 {
    if issues.is_empty() {
        return MealCompilationDispositionV1::Compiled;
    }
    if issues.iter().any(|issue| {
        matches!(
            issue.code,
            MealCompilationIssueCodeV1::DuplicateIngredientId
                | MealCompilationIssueCodeV1::DuplicateTaskId
                | MealCompilationIssueCodeV1::DuplicateFactId
                | MealCompilationIssueCodeV1::DuplicateTaskBinding
                | MealCompilationIssueCodeV1::UnknownTaskBinding
                | MealCompilationIssueCodeV1::TaskIdMismatch
                | MealCompilationIssueCodeV1::DependencyBindingMismatch
                | MealCompilationIssueCodeV1::DurationOutsideProposalWindow
                | MealCompilationIssueCodeV1::DuplicateTaskOutput
                | MealCompilationIssueCodeV1::DuplicateTaskIngredientFlow
                | MealCompilationIssueCodeV1::DuplicateFlowId
                | MealCompilationIssueCodeV1::DuplicateMaterialRef
                | MealCompilationIssueCodeV1::FlowQuantityMismatch
                | MealCompilationIssueCodeV1::InputQuantityExceedsProposal
                | MealCompilationIssueCodeV1::InputQuantityArithmeticOverflow
                | MealCompilationIssueCodeV1::TaskIngredientLineageMismatch
                | MealCompilationIssueCodeV1::OutputIngredientLineageMismatch
                | MealCompilationIssueCodeV1::InvalidProposalDigest
                | MealCompilationIssueCodeV1::SourceResolutionMismatch
                | MealCompilationIssueCodeV1::ProposalAcceptanceMismatch
                | MealCompilationIssueCodeV1::DuplicateTerminalOutputRef
        )
    }) {
        return MealCompilationDispositionV1::Conflicted;
    }
    if issues.iter().any(|issue| {
        matches!(
            issue.code,
            MealCompilationIssueCodeV1::UnsupportedSchemaVersion
                | MealCompilationIssueCodeV1::InvalidClaimCeiling
                | MealCompilationIssueCodeV1::InvalidProposal
                | MealCompilationIssueCodeV1::UnknownIngredientReference
                | MealCompilationIssueCodeV1::UnknownFactReference
                | MealCompilationIssueCodeV1::InvalidQuantity
                | MealCompilationIssueCodeV1::QuantityStateMismatch
                | MealCompilationIssueCodeV1::InvalidFactTimeWindow
                | MealCompilationIssueCodeV1::InvalidAllergenFactScope
                | MealCompilationIssueCodeV1::FactIngredientMismatch
                | MealCompilationIssueCodeV1::UnknownFactIngredient
                | MealCompilationIssueCodeV1::TaskOutputLineageIncomplete
                | MealCompilationIssueCodeV1::DependencyCycle
                | MealCompilationIssueCodeV1::ProposalRejected
                | MealCompilationIssueCodeV1::IngredientAllocationIncomplete
                | MealCompilationIssueCodeV1::InsufficientSourceRights
                | MealCompilationIssueCodeV1::FlowEndpointMismatch
                | MealCompilationIssueCodeV1::UnknownFlowIngredient
                | MealCompilationIssueCodeV1::UnknownFlowTask
                | MealCompilationIssueCodeV1::UnknownFlowOutput
                | MealCompilationIssueCodeV1::InvalidTaskOutput
                | MealCompilationIssueCodeV1::InvalidCompiledRecipeGraph
        )
    }) {
        return MealCompilationDispositionV1::Blocked;
    }
    MealCompilationDispositionV1::Indeterminate
}

fn proposal_digest(proposal: &MealProposalV1) -> serde_json::Result<String> {
    let bytes = serde_json::to_vec(proposal)?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join(""))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QuantityConversionError {
    InvalidQuantity,
    UnsupportedUnit,
    InvalidScale,
    Overflow,
}

fn quantity_to_base(
    quantity: &ExactIngredientQuantityV1,
) -> Result<(QuantityDimensionV1, u128), QuantityConversionError> {
    if !quantity.is_valid() {
        return Err(QuantityConversionError::InvalidQuantity);
    }
    let factor = match (quantity.dimension, quantity.unit) {
        (QuantityDimensionV1::Mass, QuantityUnitV1::Milligram) => 1_000_000_u128,
        (QuantityDimensionV1::Mass, QuantityUnitV1::Gram) => 1_000_000_000_u128,
        (QuantityDimensionV1::Mass, QuantityUnitV1::Kilogram) => 1_000_000_000_000_u128,
        (QuantityDimensionV1::Volume, QuantityUnitV1::Millilitre) => 1_000_000_u128,
        (QuantityDimensionV1::Volume, QuantityUnitV1::Litre) => 1_000_000_000_u128,
        (QuantityDimensionV1::Count, QuantityUnitV1::Piece | QuantityUnitV1::Count) => {
            1_000_000_u128
        }
        _ => return Err(QuantityConversionError::UnsupportedUnit),
    };
    let divisor = 10_u128
        .checked_pow(u32::from(quantity.decimal_places))
        .ok_or(())?;
    if factor % divisor != 0 {
        return Err(());
    }
    let value = u128::from(quantity.value)
        .checked_mul(factor / divisor)
        .ok_or(())?;
    Ok((quantity.dimension, value))
}

fn recipe_quantity(quantity: ExactIngredientQuantityV1) -> ExactRecipeQuantityV1 {
    ExactRecipeQuantityV1 {
        value: quantity.value,
        decimal_places: quantity.decimal_places,
        unit: match quantity.unit {
            QuantityUnitV1::Milligram => crate::recipe_graph::RecipeQuantityUnitV1::Milligram,
            QuantityUnitV1::Gram => crate::recipe_graph::RecipeQuantityUnitV1::Gram,
            QuantityUnitV1::Kilogram => crate::recipe_graph::RecipeQuantityUnitV1::Kilogram,
            QuantityUnitV1::Millilitre => crate::recipe_graph::RecipeQuantityUnitV1::Millilitre,
            QuantityUnitV1::Litre => crate::recipe_graph::RecipeQuantityUnitV1::Litre,
            QuantityUnitV1::Piece => crate::recipe_graph::RecipeQuantityUnitV1::Piece,
            QuantityUnitV1::Count => crate::recipe_graph::RecipeQuantityUnitV1::Count,
        },
    }
}

fn proposal_evidence_class(value: ProposalEvidenceClassV1) -> RecipeEvidenceClassV1 {
    match value {
        ProposalEvidenceClassV1::Modelled => RecipeEvidenceClassV1::Modelled,
        ProposalEvidenceClassV1::Simulated => RecipeEvidenceClassV1::Simulated,
        ProposalEvidenceClassV1::UserSpecified
        | ProposalEvidenceClassV1::Observed
        | ProposalEvidenceClassV1::Measured
        | ProposalEvidenceClassV1::Published
        | ProposalEvidenceClassV1::VendorDeclared => RecipeEvidenceClassV1::Declared,
    }
}

fn validate_proposal(
    proposal: &MealProposalV1,
    as_of_epoch_ms: u64,
    issues: &mut Vec<MealCompilationIssueV1>,
) {
    if proposal.schema_version != 1 {
        add_issue(
            issues,
            MealCompilationIssueCodeV1::UnsupportedSchemaVersion,
            Some("proposal.schema_version"),
            "only MealProposalV1 schema version 1 is supported",
        );
    }
    if !nonempty(&proposal.proposal_id)
        || !nonempty(&proposal.title)
        || !nonempty(&proposal.source.source_ref)
        || proposal.revision == 0
        || proposal.constraints.servings == 0
    {
        add_issue(
            issues,
            MealCompilationIssueCodeV1::InvalidProposal,
            Some(&proposal.proposal_id),
            "proposal identity, title, source, revision and serving count must be valid",
        );
    }
    if proposal.ingredients.is_empty() || proposal.tasks.is_empty() {
        add_issue(
            issues,
            MealCompilationIssueCodeV1::InvalidProposal,
            Some(&proposal.proposal_id),
            "proposal requires at least one ingredient and one task",
        );
    }
    if proposal.claim_ceiling != MEAL_PROPOSAL_CLAIM_CEILING {
        add_issue(
            issues,
            MealCompilationIssueCodeV1::InvalidClaimCeiling,
            Some("proposal.claim_ceiling"),
            "the MealProposalV1 claim ceiling must match the fixed contract",
        );
    }
    if let Some(digest) = proposal.source.content_digest_sha256.as_deref()
        && (digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    {
        add_issue(
            issues,
            MealCompilationIssueCodeV1::InvalidProposal,
            Some("proposal.source.content_digest_sha256"),
            "source content digest must be 64 lowercase hexadecimal characters",
        );
    }
    if proposal.constraints.budget_minor_units.is_some()
        && proposal.constraints.currency_code.is_none()
    {
        add_issue(
            issues,
            MealCompilationIssueCodeV1::InvalidProposal,
            Some("proposal.constraints.currency_code"),
            "a budget requires an explicit currency code",
        );
    }

    let mut ingredient_ids = BTreeSet::new();
    for ingredient in &proposal.ingredients {
        if !nonempty(&ingredient.ingredient_id)
            || !nonempty(&ingredient.display_name)
            || !ingredient_ids.insert(ingredient.ingredient_id.as_str())
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::DuplicateIngredientId,
                Some(&ingredient.ingredient_id),
                "ingredient identifiers must be non-empty and unique",
            );
        }
        match (ingredient.quantity_state, ingredient.quantity) {
            (IngredientQuantityStateV1::Explicit, Some(quantity))
                if quantity.is_valid() => {}
            (IngredientQuantityStateV1::Explicit, _) => add_issue(
                issues,
                MealCompilationIssueCodeV1::QuantityStateMismatch,
                Some(&ingredient.ingredient_id),
                "explicit ingredient quantity requires a positive, dimensionally valid quantity",
            ),
            (IngredientQuantityStateV1::Unknown | IngredientQuantityStateV1::NotRequired, Some(_)) => {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::QuantityStateMismatch,
                    Some(&ingredient.ingredient_id),
                    "unknown/not-required ingredient quantities must not contain a numeric value",
                );
            }
            (IngredientQuantityStateV1::Unknown, None) => {
                let path = format!("ingredients.{}.quantity", ingredient.ingredient_id);
                if !proposal.uncertainties.iter().any(|uncertainty| {
                    uncertainty.field_path == path && nonempty(&uncertainty.reason)
                }) {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::InvalidProposal,
                        Some(&ingredient.ingredient_id),
                        "unknown ingredient quantity requires a matching uncertainty reason",
                    );
                }
            }
            (IngredientQuantityStateV1::NotRequired, None) => {}
        }
        if ingredient
            .product_lot_ref
            .as_deref()
            .is_some_and(|value| !nonempty(value))
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::InvalidProposal,
                Some(&ingredient.ingredient_id),
                "product lot reference must be non-empty when supplied",
            );
        }
    }

    let mut task_ids = BTreeSet::new();
    for task in &proposal.tasks {
        if !nonempty(&task.task_id) || !nonempty(&task.description) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::InvalidProposal,
                Some(&task.task_id),
                "task ID and description must be non-empty",
            );
        }
        if !task_ids.insert(task.task_id.as_str()) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::DuplicateTaskId,
                Some(&task.task_id),
                "proposal task IDs must be unique",
            );
        }
        let mut task_ingredients = BTreeSet::new();
        for ingredient_ref in &task.ingredient_refs {
            if !ingredient_ids.contains(ingredient_ref.as_str()) {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::UnknownIngredientReference,
                    Some(&task.task_id),
                    format!("unknown ingredient reference {ingredient_ref}"),
                );
            }
            if !task_ingredients.insert(ingredient_ref.as_str()) {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::InvalidProposal,
                    Some(&task.task_id),
                    "task ingredient references must be unique",
                );
            }
        }
        let mut dependency_ids = BTreeSet::new();
        if task.depends_on.iter().any(|dependency| !dependency_ids.insert(dependency.as_str())) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::InvalidProposal,
                Some(&task.task_id),
                "proposal task dependency IDs must be unique",
            );
        }
        if task
            .estimated_duration_min_ms
            .zip(task.estimated_duration_max_ms)
            .is_some_and(|(minimum, maximum)| minimum > maximum)
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::InvalidProposal,
                Some(&task.task_id),
                "estimated minimum duration exceeds maximum duration",
            );
        }
        if task
            .depends_on
            .iter()
            .any(|dependency| dependency == &task.task_id)
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::DependencyCycle,
                Some(&task.task_id),
                "task cannot depend on itself",
            );
        }
    }

    for task in &proposal.tasks {
        for dependency in &task.depends_on {
            if !task_ids.contains(dependency.as_str()) {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::InvalidProposal,
                    Some(&task.task_id),
                    format!("unknown task dependency {dependency}"),
                );
            }
        }
    }

    let mut uncertainty_fields = BTreeSet::new();
    for uncertainty in &proposal.uncertainties {
        if !nonempty(&uncertainty.field_path) || !nonempty(&uncertainty.reason)
            || !uncertainty_fields.insert(uncertainty.field_path.as_str())
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::DuplicateUncertainty,
                Some(&uncertainty.field_path),
                "uncertainty field paths and reasons must be non-empty and each field path may appear once",
            );
        }
        if uncertainty.impact == UncertaintyImpactV1::PlanningBlocker {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::PlanningBlockerUnresolved,
                Some(&uncertainty.field_path),
                "an explicitly declared planning-blocking uncertainty must be resolved before compilation",
            );
        }
    }

    let mut fact_ids = BTreeSet::new();
    let mut facts = BTreeMap::new();
    for fact in &proposal.facts {
        if !nonempty(&fact.fact_id) || !fact_ids.insert(fact.fact_id.as_str()) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::DuplicateFactId,
                Some(&fact.fact_id),
                "fact identifiers must be non-empty and unique",
            );
        }
        facts.insert(fact.fact_id.as_str(), fact);
        if !nonempty(&fact.source_ref)
            || !nonempty(&fact.source_record_id)
            || !nonempty(&fact.source_version)
            || fact.observed_at_epoch_ms == 0
            || fact.value.is_null()
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::InvalidProposal,
                Some(&fact.fact_id),
                "fact source, version, observation time and non-null value are required",
            );
        }
        if fact.valid_until_epoch_ms.is_some_and(|expiry| expiry < fact.observed_at_epoch_ms) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::InvalidFactTimeWindow,
                Some(&fact.fact_id),
                "fact validity end precedes its observation time",
            );
        }
        if fact.observed_at_epoch_ms > as_of_epoch_ms
            || fact.valid_until_epoch_ms.is_some_and(|expiry| expiry < as_of_epoch_ms)
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::StaleFact,
                Some(&fact.fact_id),
                "referenced fact is not valid at the compiler as-of time",
            );
        }
        if fact
            .ingredient_id
            .as_deref()
            .is_some_and(|ingredient_id| !ingredient_ids.contains(ingredient_id))
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::UnknownFactIngredient,
                Some(&fact.fact_id),
                "fact ingredient_id must reference an ingredient in this proposal",
            );
        }
        if fact.kind == CulinaryFactKindV1::AllergenDeclaration
            && (fact
                .ingredient_id
                .as_deref()
                .is_none_or(|ingredient_id| !ingredient_ids.contains(ingredient_id))
                || !matches!(fact.scope, FactScopeV1::ProductSku | FactScopeV1::SupplierLot))
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::InvalidAllergenFactScope,
                Some(&fact.fact_id),
                "allergen declarations require a known ingredient reference and product-SKU or supplier-lot scope",
            );
        }
        if matches!(
            fact.kind,
            CulinaryFactKindV1::Price
                | CulinaryFactKindV1::Nutrient
                | CulinaryFactKindV1::Quality
                | CulinaryFactKindV1::ProcessProperty
        ) && (fact.unit.as_deref().is_none_or(|value| !nonempty(value))
            || fact.basis.as_deref().is_none_or(|value| !nonempty(value)))
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::InvalidProposal,
                Some(&fact.fact_id),
                "quantitative facts require explicit unit and basis",
            );
        }
        if matches!(
            fact.kind,
            CulinaryFactKindV1::Price | CulinaryFactKindV1::Availability
        ) && (fact
            .market_ref
            .as_deref()
            .is_none_or(|value| !nonempty(value))
            || fact.valid_until_epoch_ms.is_none())
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::InvalidProposal,
                Some(&fact.fact_id),
                "price and availability facts require a market reference and an explicit validity end",
            );
        }
    }

    for ingredient in &proposal.ingredients {
        for fact_ref in &ingredient.fact_refs {
            if let Some(fact) = facts.get(fact_ref.as_str()) {
                if fact.ingredient_id.as_deref().is_some_and(|id| id != ingredient.ingredient_id) {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::FactIngredientMismatch,
                        Some(fact_ref),
                        "an ingredient cannot consume a product/lot fact bound to a different ingredient",
                    );
                }
            } else {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::UnknownFactReference,
                    Some(&ingredient.ingredient_id),
                    format!("unknown fact reference {fact_ref}"),
                );
            }
        }
    }
    for task in &proposal.tasks {
        for fact_ref in &task.fact_refs {
            if let Some(fact) = facts.get(fact_ref.as_str()) {
                if fact.ingredient_id.as_deref().is_some_and(|id| {
                    !task.ingredient_refs.iter().any(|ingredient_ref| ingredient_ref == id)
                }) {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::FactIngredientMismatch,
                        Some(fact_ref),
                        "task evidence may reference only facts attached to one of its declared ingredients",
                    );
                }
            } else {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::UnknownFactReference,
                    Some(&task.task_id),
                    format!("unknown fact reference {fact_ref}"),
                );
            }
        }
    }

    let mut indegree: BTreeMap<&str, usize> = task_ids.iter().map(|id| (*id, 0)).collect();
    let mut children: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for task in &proposal.tasks {
        for dependency in &task.depends_on {
            if task_ids.contains(dependency.as_str()) {
                children
                    .entry(dependency.as_str())
                    .or_default()
                    .insert(task.task_id.as_str());
                if let Some(degree) = indegree.get_mut(task.task_id.as_str()) {
                    *degree += 1;
                }
            }
        }
    }
    let mut ready: BTreeSet<&str> = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
        .collect();
    let mut visited = 0_usize;
    while let Some(task_id) = ready.pop_first() {
        visited += 1;
        if let Some(next) = children.get(task_id) {
            for child in next {
                if let Some(degree) = indegree.get_mut(child) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        ready.insert(child);
                    }
                }
            }
        }
    }
    if visited != task_ids.len() {
        add_issue(
            issues,
            MealCompilationIssueCodeV1::DependencyCycle,
            Some(&proposal.proposal_id),
            "proposal task dependencies contain a cycle",
        );
    }

    match proposal.review_state {
        ProposalReviewStateV1::Accepted => {}
        ProposalReviewStateV1::Rejected => add_issue(
            issues,
            MealCompilationIssueCodeV1::ProposalRejected,
            Some(&proposal.proposal_id),
            "rejected proposals cannot be compiled",
        ),
        ProposalReviewStateV1::Proposed | ProposalReviewStateV1::HumanReviewed => {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::ProposalNotAccepted,
                Some(&proposal.proposal_id),
                "only an explicitly accepted proposal can become a compiled graph",
            );
        }
    }
}

fn validate_task_bindings(
    proposal: &MealProposalV1,
    bindings: &[MealTaskCompilationBindingV1],
    issues: &mut Vec<MealCompilationIssueV1>,
) -> BTreeMap<String, MealTaskCompilationBindingV1> {
    let proposals: BTreeMap<&str, &ProposedTaskV1> = proposal
        .tasks
        .iter()
        .map(|task| (task.task_id.as_str(), task))
        .collect();
    let mut by_id = BTreeMap::new();

    for binding in bindings {
        if by_id.insert(binding.task_id.as_str(), binding).is_some() {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::DuplicateTaskBinding,
                Some(&binding.task_id),
                "task compilation binding is duplicated",
            );
        }
        let Some(task) = proposals.get(binding.task_id.as_str()) else {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::UnknownTaskBinding,
                Some(&binding.task_id),
                "binding refers to a task absent from the proposal",
            );
            continue;
        };
        if binding.schedule.task_id != task.task_id {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::TaskIdMismatch,
                Some(&binding.task_id),
                "scheduler task ID must equal the proposal task ID",
            );
        }
        let expected: BTreeSet<&str> = task.depends_on.iter().map(String::as_str).collect();
        let actual: BTreeSet<&str> = binding
            .schedule
            .dependencies
            .iter()
            .map(|dependency| dependency.predecessor_task_id.as_str())
            .collect();
        if expected != actual || actual.len() != binding.schedule.dependencies.len() {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::DependencyBindingMismatch,
                Some(&binding.task_id),
                "scheduler dependency IDs must exactly match the proposal dependency set",
            );
        }
        if binding.schedule.duration_ms == 0
            || task
                .estimated_duration_min_ms
                .is_some_and(|minimum| binding.schedule.duration_ms < minimum)
            || task
                .estimated_duration_max_ms
                .is_some_and(|maximum| binding.schedule.duration_ms > maximum)
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::DurationOutsideProposalWindow,
                Some(&binding.task_id),
                "resolved duration must be positive and within any proposal estimate window",
            );
        }
        if !nonempty(&binding.action_resolution_ref) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::MissingActionResolution,
                Some(&binding.task_id),
                "a versioned ontology or reviewed action-resolution reference is required",
            );
        }
        if !nonempty(&binding.duration_method_ref) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::MissingDurationEvidence,
                Some(&binding.task_id),
                "a duration method or evidence reference is required",
            );
        }
        if !nonempty(&binding.schedule_resolution_ref) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::MissingDurationEvidence,
                Some(&binding.task_id),
                "an explicit schedule and dependency-lag resolution reference is required",
            );
        }
        if !nonempty(&binding.resource_resolution_ref) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::MissingResourceResolution,
                Some(&binding.task_id),
                "an explicit planning-resource resolution reference is required",
            );
        }
        if binding.schedule.resource_ids.is_empty()
            || binding.schedule.resource_ids.iter().any(|id| !nonempty(id))
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::MissingPlanningResource,
                Some(&binding.task_id),
                "at least one explicit planning resource ID is required",
            );
        }
        if let Some(ontology) = &binding.ontology_binding {
            if ontology.mapping.method == ActionMappingMethodV1::Unmapped
                || ontology.mapping.core.is_none()
                || ontology.mapping.group.is_none()
                || ontology
                    .mapping
                    .provenance_ref
                    .as_deref()
                    .is_none_or(|reference| {
                        !nonempty(reference) || reference != binding.action_resolution_ref
                    })
            {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::MissingActionOntology,
                    Some(&binding.task_id),
                    "action mapping must be mapped, versioned and bound to its exact provenance reference",
                );
            }
        } else {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::MissingActionOntology,
                Some(&binding.task_id),
                "canonical action ontology binding is required; descriptions are not executable actions",
            );
        }
        if binding.action_kind == RecipeActionKindV1::Other
            && binding
                .action_label
                .as_deref()
                .or(task.action_label.as_deref())
                .is_none_or(|label| !nonempty(label))
        {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::MissingActionResolution,
                Some(&binding.task_id),
                "other actions require a separately reviewed non-empty action label",
            );
        }
        let mut output_ids = BTreeSet::new();
        for output in &binding.outputs {
            if !nonempty(&output.output_ref)
                || !output_ids.insert(output.output_ref.as_str())
            {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::DuplicateTaskOutput,
                    Some(&binding.task_id),
                    "task output references must be non-empty and unique",
                );
            }
            if !output.quantity.is_valid() || !nonempty(&output.quantity_basis_ref) {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::InvalidTaskOutput,
                    Some(&binding.task_id),
                    "planned task outputs require a positive exact quantity and an explicit basis reference",
                );
            }
            let mut origins = BTreeSet::new();
            if output.ingredient_origins.is_empty() {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::OutputIngredientLineageMismatch,
                    Some(&output.output_ref),
                    "every planned output must carry at least one explicit ingredient-origin reference",
                );
            }
            if output
                .ingredient_origins
                .iter()
                .any(|origin| {
                    !nonempty(origin)
                        || !origins.insert(origin.as_str())
                        || !task.ingredient_refs.iter().any(|input| input.as_str() == origin.as_str())
                })
            {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::OutputIngredientLineageMismatch,
                    Some(&binding.task_id),
                    "output ingredient origins must be unique and declared by the producing proposal task",
                );
            }
        }
    }

    for task in &proposal.tasks {
        if !by_id.contains_key(task.task_id.as_str()) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::MissingTaskBinding,
                Some(&task.task_id),
                "proposal task has no explicit Symtropy compilation binding",
            );
        }
    }
    by_id
        .into_iter()
        .map(|(task_id, binding)| (task_id.to_owned(), (*binding).clone()))
        .collect()
}

fn validate_food_flows(
    proposal: &MealProposalV1,
    bindings: &BTreeMap<String, MealTaskCompilationBindingV1>,
    edges: &[FoodFlowEdgeV1],
    issues: &mut Vec<MealCompilationIssueV1>,
) {
    let ingredients: BTreeMap<&str, &ProposalIngredientV1> = proposal
        .ingredients
        .iter()
        .map(|ingredient| (ingredient.ingredient_id.as_str(), ingredient))
        .collect();
    let tasks: BTreeMap<&str, &ProposedTaskV1> = proposal
        .tasks
        .iter()
        .map(|task| (task.task_id.as_str(), task))
        .collect();
    let mut edge_ids = BTreeSet::new();
    let mut material_refs = BTreeSet::new();
    let mut ingredient_totals: BTreeMap<String, (QuantityDimensionV1, u128)> = BTreeMap::new();
    let mut output_use_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut terminal_output_refs = BTreeSet::new();
    let mut task_input_origins: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut direct_task_ingredient_inputs: BTreeSet<(String, String)> = BTreeSet::new();
    let mut task_output_origins: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for edge in edges {
        if !nonempty(&edge.edge_id) || !edge_ids.insert(edge.edge_id.as_str()) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::DuplicateFlowId,
                Some(&edge.edge_id),
                "food-flow edge IDs must be non-empty and unique",
            );
        }
        if !nonempty(&edge.material_ref) || !material_refs.insert(edge.material_ref.as_str()) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::DuplicateMaterialRef,
                Some(&edge.material_ref),
                "material references must be non-empty and unique in V1; splitting requires explicit child materials",
            );
        }
        if !edge.quantity.is_valid() || !nonempty(&edge.evidence_ref) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::InvalidTaskOutput,
                Some(&edge.edge_id),
                "flow edges require a positive exact quantity and evidence reference",
            );
            continue;
        }

        match (&edge.source, &edge.destination) {
            (
                FoodFlowSourceV1::Ingredient { ingredient_id },
                FoodFlowDestinationV1::TaskInput { task_id },
            ) => {
                let (Some(ingredient), Some(task)) = (
                    ingredients.get(ingredient_id.as_str()),
                    tasks.get(task_id.as_str()),
                ) else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownFlowIngredient,
                        Some(&edge.edge_id),
                        "ingredient-to-task flow references an unknown ingredient or task",
                    );
                    continue;
                };
                if !task
                    .ingredient_refs
                    .iter()
                    .any(|reference| reference == ingredient_id)
                {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::FlowEndpointMismatch,
                        Some(&edge.edge_id),
                        "task input must be declared by that proposal task",
                    );
                }
                let Some(declared) = ingredient.quantity else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownQuantity,
                        Some(ingredient_id),
                        "ingredient quantity is unknown or not required; an exact source allocation cannot be bounded",
                    );
                    continue;
                };
                if ingredient.quantity_state != IngredientQuantityStateV1::Explicit {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownQuantity,
                        Some(ingredient_id),
                        "only explicitly quantified ingredients may enter a compiled material flow",
                    );
                    continue;
                }
                let (Ok((declared_dimension, declared_base)), Ok((flow_dimension, flow_base))) =
                    (quantity_to_base(&declared), quantity_to_base(&edge.quantity))
                else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::InvalidQuantity,
                        Some(&edge.edge_id),
                        "ingredient and flow quantities must have exact supported unit conversions",
                    );
                    continue;
                };
                if declared_dimension != flow_dimension {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::FlowQuantityMismatch,
                        Some(&edge.edge_id),
                        "ingredient source and allocated flow dimensions differ",
                    );
                    continue;
                }
                let entry = ingredient_totals
                    .entry(ingredient_id.clone())
                    .or_insert((flow_dimension, 0));
                if entry.0 != flow_dimension {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::FlowQuantityMismatch,
                        Some(&edge.edge_id),
                        "ingredient allocations use incompatible quantity dimensions",
                    );
                    continue;
                }
                match entry.1.checked_add(flow_base) {
                    Some(total) if total <= declared_base => entry.1 = total,
                    Some(_) => add_issue(
                        issues,
                        MealCompilationIssueCodeV1::InputQuantityExceedsProposal,
                        Some(ingredient_id),
                        "allocated input quantity exceeds the proposal's explicit ingredient amount",
                    ),
                    None => add_issue(
                        issues,
                        MealCompilationIssueCodeV1::InputQuantityArithmeticOverflow,
                        Some(ingredient_id),
                        "checked input quantity aggregation overflowed",
                    ),
                }
                if !direct_task_ingredient_inputs.insert((task_id.clone(), ingredient_id.clone())) {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::DuplicateTaskIngredientFlow,
                        Some(&edge.edge_id),
                        "V1 supports one direct raw-material input edge per ingredient and task",
                    );
                }
                task_input_origins
                    .entry(task_id.clone())
                    .or_default()
                    .insert(ingredient_id.clone());
            }
            (
                FoodFlowSourceV1::TaskOutput { task_id: producer, output_ref },
                FoodFlowDestinationV1::TaskInput { task_id: consumer },
            ) => {
                let Some(consumer_task) = tasks.get(consumer.as_str()) else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownFlowTask,
                        Some(&edge.edge_id),
                        "intermediate flow references an unknown consumer task",
                    );
                    continue;
                };
                if !tasks.contains_key(producer.as_str()) {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownFlowTask,
                        Some(&edge.edge_id),
                        "intermediate flow references an unknown producer task",
                    );
                    continue;
                }
                let Some(consumer_task) = Some(*consumer_task) else {
                    unreachable!();
                };
                let Some(consumer_task) = Some(consumer_task) else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownFlowTask,
                        Some(&edge.edge_id),
                        "intermediate flow references an unknown producer or consumer task",
                    );
                    continue;
                };
                let Some(binding) = bindings.get(producer.as_str()) else {
                    continue;
                };
                let Some(output) = binding.outputs.iter().find(|item| item.output_ref == *output_ref)
                else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownFlowOutput,
                        Some(&edge.edge_id),
                        "intermediate flow references an undeclared producer output",
                    );
                    continue;
                };
                if producer == consumer
                    || !consumer_task.depends_on.iter().any(|dependency| dependency == producer)
                {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::FlowEndpointMismatch,
                        Some(&edge.edge_id),
                        "intermediate material may flow only from an explicitly declared predecessor task",
                    );
                }
                if output.quantity != edge.quantity {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::FlowQuantityMismatch,
                        Some(&edge.edge_id),
                        "intermediate flow quantity must equal its explicit producer-output quantity",
                    );
                }
                task_output_origins
                    .entry(producer.clone())
                    .or_default()
                    .extend(output.ingredient_origins.iter().cloned());
                output_use_counts
                    .entry((producer.clone(), output_ref.clone()))
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
                let _ = (producer_task, consumer_task);
                task_input_origins
                    .entry(consumer.clone())
                    .or_default()
                    .extend(output.ingredient_origins.iter().cloned());
            }
            (
                FoodFlowSourceV1::TaskOutput { task_id: producer, output_ref },
                FoodFlowDestinationV1::PlannedEdibleOutput { output_ref: destination_ref },
            )
            | (
                FoodFlowSourceV1::TaskOutput { task_id: producer, output_ref },
                FoodFlowDestinationV1::PlannedLossOutput { output_ref: destination_ref, .. },
            ) => {
                let Some(binding) = bindings.get(producer) else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownFlowTask,
                        Some(&edge.edge_id),
                        "planned output flow references an unknown producer task",
                    );
                    continue;
                };
                let Some(output) = binding.outputs.iter().find(|item| item.output_ref == *output_ref)
                else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownFlowOutput,
                        Some(&edge.edge_id),
                        "planned output flow references an undeclared task output",
                    );
                    continue;
                };
                if output.quantity != edge.quantity || !nonempty(destination_ref) {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::FlowQuantityMismatch,
                        Some(&edge.edge_id),
                        "terminal flow quantity must equal the explicit producer-output quantity and have a destination reference",
                    );
                }
                if !terminal_output_refs.insert(destination_ref.as_str()) {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::DuplicateTerminalOutputRef,
                        Some(destination_ref),
                        "terminal edible/loss destination references must be unique",
                    );
                }
                task_output_origins
                    .entry(producer.clone())
                    .or_default()
                    .extend(output.ingredient_origins.iter().cloned());
                output_use_counts
                    .entry((producer.clone(), output_ref.clone()))
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            }
            (
                FoodFlowSourceV1::Ingredient { ingredient_id },
                FoodFlowDestinationV1::PlannedLossOutput { output_ref, .. },
            ) => {
                let Some(ingredient) = ingredients.get(ingredient_id.as_str()) else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownFlowIngredient,
                        Some(&edge.edge_id),
                        "planned ingredient loss references an unknown ingredient",
                    );
                    continue;
                };
                if !nonempty(output_ref) || !terminal_output_refs.insert(output_ref.as_str()) {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::DuplicateTerminalOutputRef,
                        Some(output_ref),
                        "planned loss destination reference must be non-empty and unique",
                    );
                }
                let Some(declared) = ingredient.quantity else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::UnknownQuantity,
                        Some(ingredient_id),
                        "a planned loss cannot be quantitatively compiled from an unknown ingredient quantity",
                    );
                    continue;
                };
                let (Ok((declared_dimension, declared_base)), Ok((flow_dimension, flow_base))) =
                    (quantity_to_base(&declared), quantity_to_base(&edge.quantity))
                else {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::InvalidQuantity,
                        Some(&edge.edge_id),
                        "planned ingredient loss requires exact supported quantity conversion",
                    );
                    continue;
                };
                if ingredient.quantity_state != IngredientQuantityStateV1::Explicit
                    || declared_dimension != flow_dimension
                {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::FlowQuantityMismatch,
                        Some(&edge.edge_id),
                        "planned loss must use the same explicitly declared ingredient quantity dimension",
                    );
                    continue;
                }
                let entry = ingredient_totals
                    .entry(ingredient_id.clone())
                    .or_insert((flow_dimension, 0));
                if entry.0 != flow_dimension {
                    add_issue(
                        issues,
                        MealCompilationIssueCodeV1::FlowQuantityMismatch,
                        Some(&edge.edge_id),
                        "ingredient allocations use incompatible quantity dimensions",
                    );
                    continue;
                }
                match entry.1.checked_add(flow_base) {
                    Some(total) if total <= declared_base => entry.1 = total,
                    Some(_) => add_issue(
                        issues,
                        MealCompilationIssueCodeV1::InputQuantityExceedsProposal,
                        Some(ingredient_id),
                        "task inputs plus planned losses exceed the proposal's explicit ingredient amount",
                    ),
                    None => add_issue(
                        issues,
                        MealCompilationIssueCodeV1::InputQuantityArithmeticOverflow,
                        Some(ingredient_id),
                        "checked ingredient allocation aggregation overflowed",
                    ),
                }
            }
            (
                FoodFlowSourceV1::Ingredient { .. },
                FoodFlowDestinationV1::PlannedEdibleOutput { .. },
            ) => add_issue(
                issues,
                MealCompilationIssueCodeV1::FlowEndpointMismatch,
                Some(&edge.edge_id),
                "raw ingredient sources must enter an explicit task before becoming planned outputs",
            ),
        }
    }

    for (ingredient_id, ingredient) in &ingredients {
        if ingredient.quantity_state == IngredientQuantityStateV1::Explicit {
            if let Some(declared) = ingredient.quantity {
                if let Ok((dimension, declared_base)) = quantity_to_base(&declared) {
                    let allocated = ingredient_totals
                        .get(*ingredient_id)
                        .copied()
                        .unwrap_or((dimension, 0));
                    if allocated.0 != dimension {
                        add_issue(
                            issues,
                            MealCompilationIssueCodeV1::FlowQuantityMismatch,
                            Some(ingredient_id),
                            "ingredient allocation dimension differs from its declared dimension",
                        );
                    } else if allocated.1 < declared_base {
                        add_issue(
                            issues,
                            MealCompilationIssueCodeV1::IngredientAllocationIncomplete,
                            Some(ingredient_id),
                            "the complete explicit ingredient quantity must be allocated to task inputs and/or explicit planned losses",
                        );
                    }
                }
            }
        }
    }

    for task in &proposal.tasks {
        let actual = task_input_origins
            .get(&task.task_id)
            .cloned()
            .unwrap_or_default();
        let expected: BTreeSet<String> = task.ingredient_refs.iter().cloned().collect();
        if !expected.is_subset(&actual) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::MissingTaskIngredientFlow,
                Some(&task.task_id),
                "one or more declared task ingredients have no explicit incoming material-flow lineage",
            );
        }
        if !actual.is_subset(&expected) {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::TaskIngredientLineageMismatch,
                Some(&task.task_id),
                "material-flow lineage contains an ingredient not declared by the proposal task",
            );
        }
    }

    for task in &proposal.tasks {
        let expected: BTreeSet<String> = task.ingredient_refs.iter().cloned().collect();
        let actual = task_output_origins
            .get(&task.task_id)
            .cloned()
            .unwrap_or_default();
        if actual != expected {
            add_issue(
                issues,
                MealCompilationIssueCodeV1::TaskOutputLineageIncomplete,
                Some(&task.task_id),
                "the union of all planned task-output origins must exactly cover the task's declared ingredient inputs",
            );
        }
    }

    for (task_id, binding) in bindings {
        for output in &binding.outputs {
            let count = output_use_counts
                .get(&(task_id.clone(), output.output_ref.clone()))
                .copied()
                .unwrap_or(0);
            if count != 1 {
                add_issue(
                    issues,
                    MealCompilationIssueCodeV1::MissingTaskOutputFlow,
                    Some(&format!("{task_id}/{}", output.output_ref)),
                    "each declared task output must have exactly one downstream, edible-output or planned-loss edge; splitting is not implicit",
                );
            }
        }
    }
}

fn compilation_resolution_refs(request: &MealCompilationRequestV1) -> Vec<String> {
    let mut refs = BTreeSet::new();
    if nonempty(&request.acceptance.acceptance_ref) {
        refs.insert(request.acceptance.acceptance_ref.clone());
    }
    if nonempty(&request.acceptance.accepted_by_ref) {
        refs.insert(request.acceptance.accepted_by_ref.clone());
    }
    if nonempty(&request.proposal.source.source_ref) {
        refs.insert(request.proposal.source.source_ref.clone());
    }
    if nonempty(&request.source_resolution.reviewed_resolution_ref) {
        refs.insert(request.source_resolution.reviewed_resolution_ref.clone());
    }
    for binding in &request.task_bindings {
        for reference in [
            &binding.action_resolution_ref,
            &binding.duration_method_ref,
            &binding.schedule_resolution_ref,
            &binding.resource_resolution_ref,
        ] {
            if nonempty(reference) {
                refs.insert(reference.clone());
            }
        }
        for output in &binding.outputs {
            if nonempty(&output.quantity_basis_ref) {
                refs.insert(output.quantity_basis_ref.clone());
            }
            // ingredient_origins are identifiers, not independent evidence references.

        }
    }
    for edge in &request.food_flow_edges {
        if nonempty(&edge.evidence_ref) {
            refs.insert(edge.evidence_ref.clone());
        }
    }
    for fact in &request.proposal.facts {
        if nonempty(&fact.source_ref) {
            refs.insert(fact.source_ref.clone());
        }
        if nonempty(&fact.source_record_id) {
            refs.insert(fact.source_record_id.clone());
        }
        if nonempty(&fact.source_version) {
            refs.insert(fact.source_version.clone());
        }
    }
    refs.into_iter().collect()
}

fn build_recipe_graph(
    proposal: &MealProposalV1,
    source_resolution: &MealSourceResolutionV1,
    bindings: &BTreeMap<String, MealTaskCompilationBindingV1>,
    flow_edges: &[FoodFlowEdgeV1],
) -> RecipeTaskGraphV1 {
    let mut ingredients: Vec<RecipeIngredientV1> = proposal
        .ingredients
        .iter()
        .map(|ingredient| RecipeIngredientV1 {
            ingredient_id: ingredient.ingredient_id.clone(),
            display_name: ingredient.display_name.clone(),
            product_lot_ref: ingredient.product_lot_ref.clone(),
        })
        .collect();
    ingredients.sort_by(|left, right| left.ingredient_id.cmp(&right.ingredient_id));

    let mut tasks = Vec::new();
    for task in &proposal.tasks {
        let binding = &bindings[&task.task_id];
        let mut ingredient_inputs: Vec<RecipeIngredientUseV1> = flow_edges
            .iter()
            .filter_map(|edge| match (&edge.source, &edge.destination) {
                (
                    FoodFlowSourceV1::Ingredient { ingredient_id },
                    FoodFlowDestinationV1::TaskInput { task_id },
                ) if task_id == &task.task_id => Some(RecipeIngredientUseV1 {
                    ingredient_id: ingredient_id.clone(),
                    quantity: recipe_quantity(edge.quantity),
                }),
                _ => None,
            })
            .collect();
        ingredient_inputs.sort_by(|left, right| left.ingredient_id.cmp(&right.ingredient_id));

        let schedule = binding.schedule.clone();
        let mut output_refs: Vec<String> = binding
            .outputs
            .iter()
            .map(|output| output.output_ref.clone())
            .collect();
        output_refs.sort();

        tasks.push(RecipeTaskNodeV1 {
            schedule,
            action_kind: binding.action_kind,
            action_label: binding
                .action_label
                .clone()
                .or_else(|| task.action_label.clone()),
            ingredient_inputs,
            output_refs,
            instruction_ref: binding.instruction_ref.clone(),
            ontology_binding: binding.ontology_binding.clone(),
        });
    }
    tasks.sort_by(|left, right| left.schedule.task_id.cmp(&right.schedule.task_id));

    let mut permitted_uses = source_resolution.permitted_uses.clone();
    permitted_uses.sort();
    RecipeTaskGraphV1 {
        schema_version: RECIPE_TASK_GRAPH_SCHEMA_VERSION,
        recipe_id: proposal.proposal_id.clone(),
        revision: proposal.revision,
        title: proposal.title.clone(),
        source: RecipeSourceV1 {
            source_ref: proposal.source.source_ref.clone(),
            rights_status: source_resolution.rights_status,
            permitted_uses,
            permission_evidence_ref: source_resolution.permission_evidence_ref.clone(),
            content_sha256: proposal.source.content_digest_sha256.clone(),
            attribution_ref: proposal.source.attribution.clone(),
        },
        evidence_class: proposal_evidence_class(proposal.source.evidence_class),
        review_state: RecipeReviewStateV1::Reviewed,
        ingredients,
        tasks,
        claim_ceiling: RECIPE_GRAPH_CLAIM_CEILING.to_owned(),
    }
}

/// Compile a fully accepted proposal only when every task, schedule, action,
/// source-rights and material-flow resolution is explicit and internally
/// consistent. This function does not resolve stock, hardware or actuation.
pub fn compile_meal(request: &MealCompilationRequestV1) -> MealCompilationReportV1 {
    let mut issues = Vec::new();
    if request.schema_version != MEAL_COMPILATION_REQUEST_SCHEMA_VERSION {
        add_issue(
            &mut issues,
            MealCompilationIssueCodeV1::UnsupportedSchemaVersion,
            Some("schema_version"),
            "unsupported meal compilation request schema version",
        );
    }
    if !nonempty(&request.request_id) || request.compiled_at_epoch_ms == 0 {
        add_issue(
            &mut issues,
            MealCompilationIssueCodeV1::MissingRequiredValue,
            Some("request"),
            "request ID and explicit as-of timestamp are required",
        );
    }
    if request.claim_ceiling != MEAL_COMPILATION_CLAIM_CEILING {
        add_issue(
            &mut issues,
            MealCompilationIssueCodeV1::InvalidClaimCeiling,
            Some("claim_ceiling"),
            "meal compilation claim ceiling must match the fixed contract",
        );
    }

    let calculated_digest = proposal_digest(&request.proposal).unwrap_or_default();
    if request.proposal_sha256 != calculated_digest
        || request.proposal_sha256.len() != 64
        || !request
            .proposal_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        add_issue(
            &mut issues,
            MealCompilationIssueCodeV1::InvalidProposalDigest,
            Some("proposal_sha256"),
            "proposal SHA-256 must match the canonical typed proposal serialization",
        );
    }

    validate_proposal(&request.proposal, request.compiled_at_epoch_ms, &mut issues);

    let acceptance = &request.acceptance;
    if !nonempty(&acceptance.accepted_by_ref) || !nonempty(&acceptance.acceptance_ref) {
        add_issue(
            &mut issues,
            MealCompilationIssueCodeV1::ProposalAcceptanceMissing,
            Some("acceptance"),
            "an explicit acceptance record, reviewer reference and acceptance evidence reference are required",
        );
    }
    if acceptance.proposal_sha256 != calculated_digest
        || acceptance.proposal_revision != request.proposal.revision
        || acceptance.accepted_at_epoch_ms == 0
        || acceptance.accepted_at_epoch_ms > request.compiled_at_epoch_ms
    {
        add_issue(
            &mut issues,
            MealCompilationIssueCodeV1::ProposalAcceptanceMismatch,
            Some("acceptance"),
            "acceptance must bind the exact proposal digest/revision and precede the compilation as-of time",
        );
    }

    let source_resolution = &request.source_resolution;
    if source_resolution.source_ref != request.proposal.source.source_ref {
        add_issue(
            &mut issues,
            MealCompilationIssueCodeV1::SourceResolutionMismatch,
            Some("source_resolution.source_ref"),
            "source-rights resolution must bind the exact proposal source reference",
        );
    }
    if !nonempty(&source_resolution.reviewed_resolution_ref) {
        add_issue(
            &mut issues,
            MealCompilationIssueCodeV1::MissingSourceResolution,
            Some("source_resolution.reviewed_resolution_ref"),
            "source-rights resolution requires an auditable review reference",
        );
    }
    if !source_resolution
        .permitted_uses
        .contains(&RecipeUseScopeV1::LocalProcessing)
    {
        add_issue(
            &mut issues,
            MealCompilationIssueCodeV1::InsufficientSourceRights,
            Some(&request.proposal.proposal_id),
            "local processing must be explicitly permitted by the source-rights resolution",
        );
    }

    let bindings = validate_task_bindings(
        &request.proposal,
        &request.task_bindings,
        &mut issues,
    );
    validate_food_flows(
        &request.proposal,
        &bindings,
        &request.food_flow_edges,
        &mut issues,
    );

    let mut recipe_graph = None;
    let mut food_flow_graph = None;
    if issues.is_empty() {
        let graph = build_recipe_graph(
            &request.proposal,
            source_resolution,
            &bindings,
            &request.food_flow_edges,
        );
        let graph_report: RecipeGraphValidationReportV1 = validate_recipe_task_graph(&graph);
        if !graph_report.is_valid() {
            add_issue(
                &mut issues,
                MealCompilationIssueCodeV1::InvalidCompiledRecipeGraph,
                Some(&request.proposal.proposal_id),
                format!(
                    "compiled RecipeTaskGraphV1 failed validation with {} issue(s)",
                    graph_report.violations.len()
                ),
            );
        } else {
            let mut edges = request.food_flow_edges.clone();
            edges.sort_by(|left, right| left.edge_id.cmp(&right.edge_id));
            recipe_graph = Some(graph);
            food_flow_graph = Some(FoodFlowGraphV1 {
                schema_version: 1,
                proposal_id: request.proposal.proposal_id.clone(),
                proposal_revision: request.proposal.revision,
                edges,
                claim_ceiling: FOOD_FLOW_GRAPH_CLAIM_CEILING.to_owned(),
            });
        }
    }

    issues.sort_by(|left, right| {
        (left.code, left.reference.as_deref().unwrap_or(""), left.detail.as_str()).cmp(&(
            right.code,
            right.reference.as_deref().unwrap_or(""),
            right.detail.as_str(),
        ))
    });
    let final_disposition = disposition(&issues);
    if final_disposition != MealCompilationDispositionV1::Compiled {
        recipe_graph = None;
        food_flow_graph = None;
    }

    MealCompilationReportV1 {
        schema_version: MEAL_COMPILATION_REPORT_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        proposal_id: request.proposal.proposal_id.clone(),
        proposal_revision: request.proposal.revision,
        proposal_sha256: calculated_digest,
        compiler_version_ref: MEAL_COMPILER_VERSION_REF.to_owned(),
        disposition: final_disposition,
        issues,
        resolution_refs: compilation_resolution_refs(request),
        recipe_graph,
        food_flow_graph,
        claim_ceiling: MEAL_COMPILATION_CLAIM_CEILING.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_ontology::rule_based_action_binding;
    use crate::TaskDependencyV1;

    fn quantity(value: u64, unit: QuantityUnitV1) -> ExactIngredientQuantityV1 {
        ExactIngredientQuantityV1 {
            dimension: match unit {
                QuantityUnitV1::Milligram | QuantityUnitV1::Gram | QuantityUnitV1::Kilogram => {
                    QuantityDimensionV1::Mass
                }
                QuantityUnitV1::Millilitre | QuantityUnitV1::Litre => QuantityDimensionV1::Volume,
                QuantityUnitV1::Piece | QuantityUnitV1::Count => QuantityDimensionV1::Count,
            },
            value,
            decimal_places: 0,
            unit,
        }
    }

    fn valid_request() -> MealCompilationRequestV1 {
        let proposal = MealProposalV1 {
            schema_version: 1,
            proposal_id: "meal:onion-side".to_owned(),
            revision: 1,
            title: "Synthetic onion side".to_owned(),
            source: CulinaryProposalSourceV1 {
                source_ref: "source:synthetic-recipe".to_owned(),
                content_digest_sha256: None,
                license_ref: None,
                attribution: None,
                evidence_class: ProposalEvidenceClassV1::UserSpecified,
            },
            constraints: UserMealConstraintsV1 {
                servings: 2,
                time_budget_ms: Some(900_000),
                budget_minor_units: None,
                currency_code: None,
                preferred_cuisine_tags: Vec::new(),
                user_declared_exclusions: Vec::new(),
                user_declared_preferences: Vec::new(),
            },
            ingredients: vec![ProposalIngredientV1 {
                ingredient_id: "onion".to_owned(),
                display_name: "Onion".to_owned(),
                quantity_state: IngredientQuantityStateV1::Explicit,
                quantity: Some(quantity(250, QuantityUnitV1::Gram)),
                food_state: IngredientFoodStateV1::Raw,
                product_lot_ref: None,
                fact_refs: Vec::new(),
            }],
            tasks: vec![
                ProposedTaskV1 {
                    task_id: "chop-onion".to_owned(),
                    description: "Chop the onion".to_owned(),
                    action_label: Some("chop".to_owned()),
                    ingredient_refs: vec!["onion".to_owned()],
                    depends_on: Vec::new(),
                    estimated_duration_min_ms: Some(20_000),
                    estimated_duration_max_ms: Some(40_000),
                    fact_refs: Vec::new(),
                },
                ProposedTaskV1 {
                    task_id: "saute-onion".to_owned(),
                    description: "Saute until translucent".to_owned(),
                    action_label: Some("saute".to_owned()),
                    ingredient_refs: vec!["onion".to_owned()],
                    depends_on: vec!["chop-onion".to_owned()],
                    estimated_duration_min_ms: Some(120_000),
                    estimated_duration_max_ms: Some(240_000),
                    fact_refs: Vec::new(),
                },
            ],
            facts: Vec::new(),
            uncertainties: Vec::new(),
            review_state: ProposalReviewStateV1::Accepted,
            claim_ceiling: MEAL_PROPOSAL_CLAIM_CEILING.to_owned(),
        };

        let proposal_sha256 = proposal_digest(&proposal).unwrap();

        let mut chop_binding = MealTaskCompilationBindingV1 {
            task_id: "chop-onion".to_owned(),
            action_kind: RecipeActionKindV1::Chop,
            action_label: Some("chop".to_owned()),
            schedule: KitchenTaskV1 {
                task_id: "chop-onion".to_owned(),
                duration_ms: 30_000,
                resource_ids: vec!["manual-prep".to_owned()],
                dependencies: Vec::new(),
                earliest_start_ms: Some(0),
                latest_finish_ms: Some(60_000),
            },
            ontology_binding: Some(rule_based_action_binding(
                RecipeActionKindV1::Chop,
                RecipeEvidenceClassV1::Declared,
            )),
            action_resolution_ref: "builtin:symtropy-kitchen-action-rules-v1".to_owned(),
            duration_method_ref: "method:manual-cutting-v1".to_owned(),
            schedule_resolution_ref: "schedule:manual-prep-v1".to_owned(),
            resource_resolution_ref: "resolution:manual-prep-v1".to_owned(),
            instruction_ref: Some("instruction:step-1".to_owned()),
            outputs: vec![
                PlannedTaskOutputV1 {
                    output_ref: "prepared-onion".to_owned(),
                    quantity: quantity(240, QuantityUnitV1::Gram),
                    ingredient_origins: vec!["onion".to_owned()],
                    quantity_basis_ref: "model:trim-yield-v1".to_owned(),
                },
                PlannedTaskOutputV1 {
                    output_ref: "onion-trim".to_owned(),
                    quantity: quantity(10, QuantityUnitV1::Gram),
                    ingredient_origins: vec!["onion".to_owned()],
                    quantity_basis_ref: "model:trim-yield-v1".to_owned(),
                },
            ],
        };
        let mut saute_binding = MealTaskCompilationBindingV1 {
            task_id: "saute-onion".to_owned(),
            action_kind: RecipeActionKindV1::Saute,
            action_label: Some("saute".to_owned()),
            schedule: KitchenTaskV1 {
                task_id: "saute-onion".to_owned(),
                duration_ms: 180_000,
                resource_ids: vec!["hob-zone-1".to_owned()],
                dependencies: vec![TaskDependencyV1 {
                    predecessor_task_id: "chop-onion".to_owned(),
                    min_lag_ms: 0,
                    max_lag_ms: Some(60_000),
                }],
                earliest_start_ms: None,
                latest_finish_ms: None,
            },
            ontology_binding: Some(rule_based_action_binding(
                RecipeActionKindV1::Saute,
                RecipeEvidenceClassV1::Declared,
            )),
            action_resolution_ref: "builtin:symtropy-kitchen-action-rules-v1".to_owned(),
            duration_method_ref: "method:saute-v1".to_owned(),
            schedule_resolution_ref: "schedule:hob-zone-1-v1".to_owned(),
            resource_resolution_ref: "resolution:hob-zone-1-v1".to_owned(),
            instruction_ref: Some("instruction:step-2".to_owned()),
            outputs: vec![PlannedTaskOutputV1 {
                output_ref: "sauteed-onion".to_owned(),
                quantity: quantity(200, QuantityUnitV1::Gram),
                ingredient_origins: vec!["onion".to_owned()],
                quantity_basis_ref: "model:cooking-yield-v1".to_owned(),
            }],
        };
        MealCompilationRequestV1 {
            schema_version: 1,
            request_id: "compile:onion-side:1".to_owned(),
            compiled_at_epoch_ms: 1_750_000_000_000,
            proposal_sha256,
            proposal,
            acceptance: ProposalAcceptanceV1 {
                proposal_sha256: proposal_sha256.clone(),
                proposal_revision: 1,
                accepted_by_ref: "household:reviewer-1".to_owned(),
                accepted_at_epoch_ms: 1_749_999_999_000,
                acceptance_ref: "consent:proposal-acceptance-v1".to_owned(),
            },
            source_resolution: MealSourceResolutionV1 {
                source_ref: "source:synthetic-recipe".to_owned(),
                rights_status: RecipeRightsStatusV1::UserSupplied,
                permitted_uses: vec![RecipeUseScopeV1::LocalProcessing],
                permission_evidence_ref: None,
                reviewed_resolution_ref: "review:source-rights-local-v1".to_owned(),
            },
            task_bindings: vec![chop_binding, saute_binding],
            food_flow_edges: vec![
                FoodFlowEdgeV1 {
                    edge_id: "edge:raw-onion-to-chop".to_owned(),
                    material_ref: "material:raw-onion".to_owned(),
                    source: FoodFlowSourceV1::Ingredient {
                        ingredient_id: "onion".to_owned(),
                    },
                    destination: FoodFlowDestinationV1::TaskInput {
                        task_id: "chop-onion".to_owned(),
                    },
                    quantity: quantity(250, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:raw-onion-v1".to_owned(),
                },
                FoodFlowEdgeV1 {
                    edge_id: "edge:raw-onion-trim-loss".to_owned(),
                    material_ref: "material:onion-trim".to_owned(),
                    source: FoodFlowSourceV1::TaskOutput {
                        task_id: "chop-onion".to_owned(),
                        output_ref: "onion-trim".to_owned(),
                    },
                    destination: FoodFlowDestinationV1::PlannedLossOutput {
                        output_ref: "planned-loss:onion-trim".to_owned(),
                        loss_kind: PlannedLossKindV1::Trim,
                    },
                    quantity: quantity(10, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:onion-trim-v1".to_owned(),
                },
                FoodFlowEdgeV1 {
                    edge_id: "edge:chop-to-saute".to_owned(),
                    material_ref: "material:prepared-onion".to_owned(),
                    source: FoodFlowSourceV1::TaskOutput {
                        task_id: "chop-onion".to_owned(),
                        output_ref: "prepared-onion".to_owned(),
                    },
                    destination: FoodFlowDestinationV1::TaskInput {
                        task_id: "saute-onion".to_owned(),
                    },
                    quantity: quantity(240, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:prepared-onion-v1".to_owned(),
                },
                FoodFlowEdgeV1 {
                    edge_id: "edge:saute-to-edible-output".to_owned(),
                    material_ref: "material:sauteed-onion".to_owned(),
                    source: FoodFlowSourceV1::TaskOutput {
                        task_id: "saute-onion".to_owned(),
                        output_ref: "sauteed-onion".to_owned(),
                    },
                    destination: FoodFlowDestinationV1::PlannedEdibleOutput {
                        output_ref: "meal-component:onion".to_owned(),
                    },
                    quantity: quantity(200, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:sauteed-onion-v1".to_owned(),
                },
            ],
            claim_ceiling: MEAL_COMPILATION_CLAIM_CEILING.to_owned(),
        }
    }

    #[test]
    fn compiles_linked_action_and_material_flow_graphs_without_recounting_inputs() {
        let request = valid_request();
        let report = compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Compiled);
        assert!(report.is_compiled(), "{:?}", report.issues);
        let graph = report.recipe_graph.unwrap();
        let chop = graph
            .tasks
            .iter()
            .find(|task| task.schedule.task_id == "chop-onion")
            .unwrap();
        let saute = graph
            .tasks
            .iter()
            .find(|task| task.schedule.task_id == "saute-onion")
            .unwrap();
        assert_eq!(chop.ingredient_inputs.len(), 1);
        assert!(saute.ingredient_inputs.is_empty());
        assert_eq!(report.food_flow_graph.unwrap().edges.len(), 3);
    }

    #[test]
    fn raw_inputs_and_explicit_trim_loss_reconcile_to_declared_quantity() {
        let report = compile_meal(&valid_request());
        assert_eq!(report.disposition, MealCompilationDispositionV1::Compiled);
        assert!(report.is_compiled());
    }

    #[test]
    fn acceptance_must_bind_exact_proposal_digest_and_revision() {
        let mut request = valid_request();
        request.acceptance.proposal_revision = 2;
        let report = compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Conflicted);
        assert!(report.issues.iter().any(|issue| {
            issue.code == MealCompilationIssueCodeV1::ProposalAcceptanceMismatch
        }));
    }

    #[test]
    fn incomplete_raw_material_allocation_is_indeterminate() {
        let mut request = valid_request();
        request.food_flow_edges.retain(|edge| edge.edge_id != "edge:raw-onion-to-chop");
        let report = compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Indeterminate);
        assert!(report.issues.iter().any(|issue| {
            issue.code == MealCompilationIssueCodeV1::IngredientAllocationIncomplete
        }));
    }

    #[test]
    fn missing_task_binding_stays_indeterminate() {
        let mut request = valid_request();
        request.task_bindings.pop();
        let report = compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Indeterminate);
        assert!(report.recipe_graph.is_none());
    }

    #[test]
    fn unknown_quantity_cannot_be_allocated_as_a_numeric_flow() {
        let mut request = valid_request();
        request.proposal.ingredients[0].quantity_state = IngredientQuantityStateV1::Unknown;
        request.proposal.ingredients[0].quantity = None;
        request.proposal.uncertainties.push(ProposalUncertaintyV1 {
            field_path: "ingredients.onion.quantity".to_owned(),
            reason: "Household amount not observed".to_owned(),
            impact: UncertaintyImpactV1::PlanningBlocker,
        });
        request.proposal_sha256 = proposal_digest(&request.proposal).unwrap();
        let report = compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Indeterminate);
        assert!(report.recipe_graph.is_none());
    }

    #[test]
    fn schedule_dependency_set_must_match_proposal_exactly() {
        let mut request = valid_request();
        request.task_bindings[1].schedule.dependencies.clear();
        let report = compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Conflicted);
        assert!(report.recipe_graph.is_none());
    }

    #[test]
    fn raw_ingredient_cannot_be_allocated_twice_beyond_proposal_quantity() {
        let mut request = valid_request();
        request.food_flow_edges.push(FoodFlowEdgeV1 {
            edge_id: "edge:second-onion-allocation".to_owned(),
            material_ref: "material:second-raw-onion".to_owned(),
            source: FoodFlowSourceV1::Ingredient {
                ingredient_id: "onion".to_owned(),
            },
            destination: FoodFlowDestinationV1::TaskInput {
                task_id: "chop-onion".to_owned(),
            },
            quantity: quantity(1, QuantityUnitV1::Kilogram),
            evidence_ref: "allocation:duplicate-onion-v1".to_owned(),
        });
        let report = compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Conflicted);
        assert!(report.issues.iter().any(|issue| {
            issue.code == MealCompilationIssueCodeV1::InputQuantityExceedsProposal
        }));
    }

    #[test]
    fn unknown_action_mapping_never_compiles_from_description() {
        let mut request = valid_request();
        request.task_bindings[0].ontology_binding = None;
        let report = compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Indeterminate);
        assert!(report.recipe_graph.is_none());
    }

    #[test]
    fn planned_loss_and_edible_outputs_remain_plans_not_measured_acceptance() {
        let mut request = valid_request();
        request.food_flow_edges[2].destination =
            FoodFlowDestinationV1::PlannedLossOutput {
                output_ref: "planned-loss:sauteed-onion".to_owned(),
                loss_kind: PlannedLossKindV1::ExpectedReject,
            };
        request.food_flow_edges[2].material_ref = "material:planned-loss-onion".to_owned();
        let report = compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Compiled);
        assert!(report.food_flow_graph.unwrap().claim_ceiling.contains("not measured process yield"));
    }
}

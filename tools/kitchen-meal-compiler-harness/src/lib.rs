// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Focused executable contract tests against the production Symtropy kitchen
//! planner crate. This harness imports production code by path dependency; it
//! does not copy or stub the compiler implementation.

pub use symtropy_kitchen_planner::meal_compiler;

#[cfg(test)]
mod tests {
    use meal_compiler::*;
    use sha2::{Digest, Sha256};
    use symtropy_kitchen_planner::action_ontology::rule_based_action_binding;
    use symtropy_kitchen_planner::recipe_graph::{
        RecipeActionKindV1, RecipeEvidenceClassV1, RecipeRightsStatusV1, RecipeUseScopeV1,
    };
    use symtropy_kitchen_planner::{KitchenTaskV1, TaskDependencyV1};

    fn qty(value: u64, unit: QuantityUnitV1) -> ExactIngredientQuantityV1 {
        ExactIngredientQuantityV1 {
            dimension: match unit {
                QuantityUnitV1::Milligram | QuantityUnitV1::Gram | QuantityUnitV1::Kilogram => QuantityDimensionV1::Mass,
                QuantityUnitV1::Millilitre | QuantityUnitV1::Litre => QuantityDimensionV1::Volume,
                QuantityUnitV1::Piece | QuantityUnitV1::Count => QuantityDimensionV1::Count,
            },
            value,
            decimal_places: 0,
            unit,
        }
    }

    fn digest(proposal: &MealProposalV1) -> String {
        Sha256::digest(serde_json::to_vec(proposal).unwrap())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join("")
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
                quantity: Some(qty(250, QuantityUnitV1::Gram)),
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
        let proposal_sha256 = digest(&proposal);

        let chop = MealTaskCompilationBindingV1 {
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
                RecipeActionKindV1::Chop, RecipeEvidenceClassV1::Declared,
            )),
            action_resolution_ref: "builtin:symtropy-kitchen-action-rules-v1".to_owned(),
            duration_method_ref: "method:manual-cutting-v1".to_owned(),
            schedule_resolution_ref: "schedule:manual-prep-v1".to_owned(),
            resource_resolution_ref: "resolution:manual-prep-v1".to_owned(),
            instruction_ref: Some("instruction:step-1".to_owned()),
            outputs: vec![
                PlannedTaskOutputV1 {
                    output_ref: "prepared-onion".to_owned(),
                    quantity: qty(240, QuantityUnitV1::Gram),
                    ingredient_origins: vec!["onion".to_owned()],
                    quantity_basis_ref: "model:trim-yield-v1".to_owned(),
                },
                PlannedTaskOutputV1 {
                    output_ref: "onion-trim".to_owned(),
                    quantity: qty(10, QuantityUnitV1::Gram),
                    ingredient_origins: vec!["onion".to_owned()],
                    quantity_basis_ref: "model:trim-yield-v1".to_owned(),
                },
            ],
        };
        let saute = MealTaskCompilationBindingV1 {
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
                RecipeActionKindV1::Saute, RecipeEvidenceClassV1::Declared,
            )),
            action_resolution_ref: "builtin:symtropy-kitchen-action-rules-v1".to_owned(),
            duration_method_ref: "method:saute-v1".to_owned(),
            schedule_resolution_ref: "schedule:hob-zone-1-v1".to_owned(),
            resource_resolution_ref: "resolution:hob-zone-1-v1".to_owned(),
            instruction_ref: Some("instruction:step-2".to_owned()),
            outputs: vec![PlannedTaskOutputV1 {
                output_ref: "sauteed-onion".to_owned(),
                quantity: qty(200, QuantityUnitV1::Gram),
                ingredient_origins: vec!["onion".to_owned()],
                quantity_basis_ref: "model:cooking-yield-v1".to_owned(),
            }],
        };

        MealCompilationRequestV1 {
            schema_version: 1,
            request_id: "compile:onion-side:1".to_owned(),
            compiled_at_epoch_ms: 1_750_000_000_000,
            proposal_sha256: proposal_sha256.clone(),
            proposal,
            acceptance: ProposalAcceptanceV1 {
                proposal_sha256,
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
            task_bindings: vec![chop, saute],
            food_flow_edges: vec![
                FoodFlowEdgeV1 {
                    edge_id: "edge:raw-onion-to-chop".to_owned(),
                    material_ref: "material:raw-onion".to_owned(),
                    source: FoodFlowSourceV1::Ingredient { ingredient_id: "onion".to_owned() },
                    destination: FoodFlowDestinationV1::TaskInput { task_id: "chop-onion".to_owned() },
                    quantity: qty(250, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:raw-onion-v1".to_owned(),
                },
                FoodFlowEdgeV1 {
                    edge_id: "edge:chop-to-saute".to_owned(),
                    material_ref: "material:prepared-onion".to_owned(),
                    source: FoodFlowSourceV1::TaskOutput { task_id: "chop-onion".to_owned(), output_ref: "prepared-onion".to_owned() },
                    destination: FoodFlowDestinationV1::TaskInput { task_id: "saute-onion".to_owned() },
                    quantity: qty(240, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:prepared-onion-v1".to_owned(),
                },
                FoodFlowEdgeV1 {
                    edge_id: "edge:raw-onion-trim-loss".to_owned(),
                    material_ref: "material:onion-trim".to_owned(),
                    source: FoodFlowSourceV1::TaskOutput { task_id: "chop-onion".to_owned(), output_ref: "onion-trim".to_owned() },
                    destination: FoodFlowDestinationV1::PlannedLossOutput {
                        output_ref: "planned-loss:onion-trim".to_owned(),
                        loss_kind: PlannedLossKindV1::Trim,
                    },
                    quantity: qty(10, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:onion-trim-v1".to_owned(),
                },
                FoodFlowEdgeV1 {
                    edge_id: "edge:saute-to-edible-output".to_owned(),
                    material_ref: "material:sauteed-onion".to_owned(),
                    source: FoodFlowSourceV1::TaskOutput { task_id: "saute-onion".to_owned(), output_ref: "sauteed-onion".to_owned() },
                    destination: FoodFlowDestinationV1::PlannedEdibleOutput { output_ref: "planned-edible:onion".to_owned() },
                    quantity: qty(200, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:sauteed-onion-v1".to_owned(),
                },
            ],
            claim_ceiling: MEAL_COMPILATION_CLAIM_CEILING.to_owned(),
        }
    }

    #[test]
    fn exact_production_compiler_compiles_both_graphs() {
        let report = meal_compiler::compile_meal(&valid_request());
        assert!(report.is_compiled(), "{:?}", report.issues);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Compiled);
        assert_eq!(report.recipe_graph.as_ref().unwrap().tasks.len(), 2);
        assert_eq!(report.food_flow_graph.as_ref().unwrap().edges.len(), 4);
    }

    #[test]
    fn proposal_acceptance_cannot_be_replayed_for_another_revision() {
        let mut request = valid_request();
        request.acceptance.proposal_revision = 2;
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Conflicted);
        assert!(report.issues.iter().any(|issue| {
            issue.code == MealCompilationIssueCodeV1::ProposalAcceptanceMismatch
        }));
    }

    #[test]
    fn unknown_household_quantity_is_not_promoted_to_a_number() {
        let mut request = valid_request();
        request.proposal.ingredients[0].quantity_state = IngredientQuantityStateV1::Unknown;
        request.proposal.ingredients[0].quantity = None;
        request.proposal.uncertainties.push(ProposalUncertaintyV1 {
            field_path: "ingredients.onion.quantity".to_owned(),
            reason: "Household quantity not observed".to_owned(),
            impact: UncertaintyImpactV1::PlanningBlocker,
        });
        request.proposal_sha256 = digest(&request.proposal);
        request.acceptance.proposal_sha256 = request.proposal_sha256.clone();
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Indeterminate);
        assert!(report.recipe_graph.is_none());
    }

    #[test]
    fn underallocated_ingredient_requires_explicit_residual_or_loss_path() {
        let mut request = valid_request();
        request.food_flow_edges.retain(|edge| edge.edge_id != "edge:raw-onion-trim-loss");
        // The 10 g trim output is still present, but all explicit 250 g raw onion
        // must be assigned to a task input or explicit planned loss.
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Indeterminate);
        assert!(report.issues.iter().any(|issue| {
            issue.code == MealCompilationIssueCodeV1::IngredientAllocationIncomplete
        }));
    }
}

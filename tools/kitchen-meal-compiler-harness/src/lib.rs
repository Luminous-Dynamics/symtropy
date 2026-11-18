// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Focused executable contract tests against the production Symtropy kitchen
//! planner crate. This harness imports production code by path dependency; it
//! does not copy or stub the compiler implementation.

pub use symtropy_kitchen_planner::meal_compiler;

#[cfg(test)]
mod tests {
    use crate::meal_compiler::{self, *};
    use sha2::{Digest, Sha256};
    use symtropy_kitchen_planner::action_ontology::rule_based_action_binding;
    use symtropy_kitchen_planner::recipe_graph::{
        RecipeActionKindV1, RecipeEvidenceClassV1, RecipeRightsStatusV1, RecipeUseScopeV1,
    };
    use symtropy_kitchen_planner::{KitchenTaskV1, TaskDependencyV1};

    fn qty(value: u64, unit: QuantityUnitV1) -> ExactIngredientQuantityV1 {
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
                    quantity: qty(240, QuantityUnitV1::Gram),
                    ingredient_origins: vec!["onion".to_owned()],
                    ingredient_contributions: vec![PlannedIngredientContributionV1 {
                        ingredient_id: "onion".to_owned(),
                        quantity: qty(240, QuantityUnitV1::Gram),
                    }],
                    quantity_basis_ref: "model:trim-yield-v1".to_owned(),
                },
                PlannedTaskOutputV1 {
                    output_ref: "onion-trim".to_owned(),
                    quantity: qty(10, QuantityUnitV1::Gram),
                    ingredient_origins: vec!["onion".to_owned()],
                    ingredient_contributions: vec![PlannedIngredientContributionV1 {
                        ingredient_id: "onion".to_owned(),
                        quantity: qty(10, QuantityUnitV1::Gram),
                    }],
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
                RecipeActionKindV1::Saute,
                RecipeEvidenceClassV1::Declared,
            )),
            action_resolution_ref: "builtin:symtropy-kitchen-action-rules-v1".to_owned(),
            duration_method_ref: "method:saute-v1".to_owned(),
            schedule_resolution_ref: "schedule:hob-zone-1-v1".to_owned(),
            resource_resolution_ref: "resolution:hob-zone-1-v1".to_owned(),
            instruction_ref: Some("instruction:step-2".to_owned()),
            outputs: vec![
                PlannedTaskOutputV1 {
                    output_ref: "sauteed-onion".to_owned(),
                    quantity: qty(200, QuantityUnitV1::Gram),
                    ingredient_origins: vec!["onion".to_owned()],
                    ingredient_contributions: vec![PlannedIngredientContributionV1 {
                        ingredient_id: "onion".to_owned(),
                        quantity: qty(200, QuantityUnitV1::Gram),
                    }],
                    quantity_basis_ref: "model:cooking-yield-v1".to_owned(),
                },
                PlannedTaskOutputV1 {
                    output_ref: "saute-loss".to_owned(),
                    quantity: qty(40, QuantityUnitV1::Gram),
                    ingredient_origins: vec!["onion".to_owned()],
                    ingredient_contributions: vec![PlannedIngredientContributionV1 {
                        ingredient_id: "onion".to_owned(),
                        quantity: qty(40, QuantityUnitV1::Gram),
                    }],
                    quantity_basis_ref: "model:cooking-loss-v1".to_owned(),
                },
            ],
        };

        MealCompilationRequestV1 {
            schema_version: 1,
            request_id: "compile:onion-side:1".to_owned(),
            compiled_at_epoch_ms: 1_750_000_000_000,
            as_of_time_evidence_ref: "clock:fixture-as-of-v1".to_owned(),
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
                    source: FoodFlowSourceV1::Ingredient {
                        ingredient_id: "onion".to_owned(),
                    },
                    destination: FoodFlowDestinationV1::TaskInput {
                        task_id: "chop-onion".to_owned(),
                    },
                    quantity: qty(250, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:raw-onion-v1".to_owned(),
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
                    quantity: qty(240, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:prepared-onion-v1".to_owned(),
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
                    quantity: qty(10, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:onion-trim-v1".to_owned(),
                },
                FoodFlowEdgeV1 {
                    edge_id: "edge:saute-to-planned-loss".to_owned(),
                    material_ref: "material:saute-loss".to_owned(),
                    source: FoodFlowSourceV1::TaskOutput {
                        task_id: "saute-onion".to_owned(),
                        output_ref: "saute-loss".to_owned(),
                    },
                    destination: FoodFlowDestinationV1::PlannedLossOutput {
                        output_ref: "planned-loss:saute-residue".to_owned(),
                        loss_kind: PlannedLossKindV1::ExpectedEvaporation,
                    },
                    quantity: qty(40, QuantityUnitV1::Gram),
                    evidence_ref: "allocation:saute-loss-v1".to_owned(),
                },
                FoodFlowEdgeV1 {
                    edge_id: "edge:saute-to-edible-output".to_owned(),
                    material_ref: "material:sauteed-onion".to_owned(),
                    source: FoodFlowSourceV1::TaskOutput {
                        task_id: "saute-onion".to_owned(),
                        output_ref: "sauteed-onion".to_owned(),
                    },
                    destination: FoodFlowDestinationV1::PlannedEdibleOutput {
                        output_ref: "planned-edible:onion".to_owned(),
                    },
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
        assert_eq!(report.food_flow_graph.as_ref().unwrap().edges.len(), 5);
    }

    #[test]
    fn planned_task_inputs_must_reconcile_to_outputs_and_explicit_losses() {
        let mut request = valid_request();
        request.task_bindings[1]
            .outputs
            .retain(|output| output.output_ref != "saute-loss");
        request
            .food_flow_edges
            .retain(|edge| edge.edge_id != "edge:saute-to-planned-loss");
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Conflicted);
        assert!(report.issues.iter().any(|issue| {
            issue.code == MealCompilationIssueCodeV1::TaskMaterialBalanceMismatch
        }));
    }

    #[test]
    fn per_ingredient_output_contributions_must_equal_the_output_quantity() {
        let mut request = valid_request();
        request.task_bindings[0].outputs[0].ingredient_contributions[0].quantity =
            qty(239, QuantityUnitV1::Gram);
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Conflicted);
        assert!(report.issues.iter().any(|issue| {
            issue.code == MealCompilationIssueCodeV1::OutputContributionTotalMismatch
        }));
    }

    #[test]
    fn one_planned_output_cannot_mix_mass_and_volume_contributions() {
        let mut request = valid_request();
        request.task_bindings[0].outputs[0].ingredient_contributions[0].quantity =
            qty(240, QuantityUnitV1::Millilitre);
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Blocked);
        assert!(report.issues.iter().any(|issue| {
            issue.code == MealCompilationIssueCodeV1::InvalidOutputContribution
        }));
    }

    #[test]
    fn proposal_acceptance_cannot_be_replayed_for_another_revision() {
        let mut request = valid_request();
        request.acceptance.proposal_revision = 2;
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Conflicted);
        assert!(
            report.issues.iter().any(|issue| {
                issue.code == MealCompilationIssueCodeV1::ProposalAcceptanceMismatch
            })
        );
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
        assert_eq!(
            report.disposition,
            MealCompilationDispositionV1::Indeterminate
        );
        assert!(report.recipe_graph.is_none());
    }

    #[test]
    fn missing_as_of_time_evidence_keeps_freshness_indeterminate() {
        let mut request = valid_request();
        request.as_of_time_evidence_ref.clear();
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(
            report.disposition,
            MealCompilationDispositionV1::Indeterminate
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == MealCompilationIssueCodeV1::MissingAsOfTimeEvidence })
        );
    }

    #[test]
    fn generic_ingredient_price_cannot_be_promoted_to_market_price() {
        let mut request = valid_request();
        request.proposal.ingredients[0]
            .fact_refs
            .push("price:onion".to_owned());
        request.proposal.facts.push(CulinaryFactV1 {
            fact_id: "price:onion".to_owned(),
            ingredient_id: Some("onion".to_owned()),
            kind: CulinaryFactKindV1::Price,
            scope: FactScopeV1::GenericIngredient,
            value: serde_json::json!({"minor_units": 250}),
            unit: Some("ZAR/kg".to_owned()),
            basis: Some("retail-pack-normalized".to_owned()),
            source_ref: "source:generic-catalogue".to_owned(),
            source_record_id: "record:onion".to_owned(),
            source_version: "revision:1".to_owned(),
            evidence_class: ProposalEvidenceClassV1::Published,
            observed_at_epoch_ms: 1_749_000_000_000,
            valid_until_epoch_ms: Some(1_751_000_000_000),
            market_ref: Some("market:johannesburg".to_owned()),
            license_ref: None,
        });
        request.proposal_sha256 = digest(&request.proposal);
        request.acceptance.proposal_sha256 = request.proposal_sha256.clone();
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Blocked);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == MealCompilationIssueCodeV1::InvalidPriceFactScope })
        );
    }

    #[test]
    fn compilation_report_preserves_time_source_lot_and_market_references() {
        let mut request = valid_request();
        request.proposal.ingredients[0].product_lot_ref =
            Some("mycelix:food-lot:onion-lot-001".to_owned());
        request.proposal.ingredients[0]
            .fact_refs
            .push("price:onion".to_owned());
        request.proposal.facts.push(CulinaryFactV1 {
            fact_id: "price:onion".to_owned(),
            ingredient_id: Some("onion".to_owned()),
            kind: CulinaryFactKindV1::Price,
            scope: FactScopeV1::ProductSku,
            value: serde_json::json!({"minor_units": 250}),
            unit: Some("ZAR/kg".to_owned()),
            basis: Some("retail-pack-normalized".to_owned()),
            source_ref: "source:retailer".to_owned(),
            source_record_id: "record:price-onion".to_owned(),
            source_version: "revision:4".to_owned(),
            evidence_class: ProposalEvidenceClassV1::Published,
            observed_at_epoch_ms: 1_749_000_000_000,
            valid_until_epoch_ms: Some(1_751_000_000_000),
            market_ref: Some("market:johannesburg".to_owned()),
            license_ref: Some("license:catalogue-v1".to_owned()),
        });
        request.proposal_sha256 = digest(&request.proposal);
        request.acceptance.proposal_sha256 = request.proposal_sha256.clone();
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(
            report.disposition,
            MealCompilationDispositionV1::Compiled,
            "{:?}",
            report.issues
        );
        for reference in [
            "clock:fixture-as-of-v1",
            "mycelix:food-lot:onion-lot-001",
            "source:retailer",
            "record:price-onion",
            "revision:4",
            "market:johannesburg",
            "license:catalogue-v1",
        ] {
            assert!(
                report.resolution_refs.iter().any(|item| item == reference),
                "missing {reference}"
            );
        }
    }

    #[test]
    fn identical_compilation_inputs_produce_identical_reports() {
        let request = valid_request();
        let first = meal_compiler::compile_meal(&request);
        let second = meal_compiler::compile_meal(&request);
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
    }

    #[test]
    fn stale_price_fact_is_not_treated_as_current() {
        let mut request = valid_request();
        request.proposal.ingredients[0]
            .fact_refs
            .push("price:onion".to_owned());
        request.proposal.facts.push(CulinaryFactV1 {
            fact_id: "price:onion".to_owned(),
            ingredient_id: Some("onion".to_owned()),
            kind: CulinaryFactKindV1::Price,
            scope: FactScopeV1::ProductSku,
            value: serde_json::json!({"minor_units": 250}),
            unit: Some("ZAR/kg".to_owned()),
            basis: Some("retail-pack-normalized".to_owned()),
            source_ref: "source:retailer".to_owned(),
            source_record_id: "record:price-onion".to_owned(),
            source_version: "revision:4".to_owned(),
            evidence_class: ProposalEvidenceClassV1::Published,
            observed_at_epoch_ms: 1_749_000_000_000,
            valid_until_epoch_ms: Some(1_749_999_999_999),
            market_ref: Some("market:johannesburg".to_owned()),
            license_ref: None,
        });
        request.proposal_sha256 = digest(&request.proposal);
        request.acceptance.proposal_sha256 = request.proposal_sha256.clone();
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(
            report.disposition,
            MealCompilationDispositionV1::Indeterminate
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == MealCompilationIssueCodeV1::StaleFact)
        );
    }

    #[test]
    fn generic_ingredient_allergen_fact_cannot_become_product_assurance() {
        let mut request = valid_request();
        request.proposal.ingredients[0]
            .fact_refs
            .push("allergen:onion".to_owned());
        request.proposal.facts.push(CulinaryFactV1 {
            fact_id: "allergen:onion".to_owned(),
            ingredient_id: Some("onion".to_owned()),
            kind: CulinaryFactKindV1::AllergenDeclaration,
            scope: FactScopeV1::GenericIngredient,
            value: serde_json::json!({"declared_allergens": []}),
            unit: None,
            basis: None,
            source_ref: "source:generic-catalogue".to_owned(),
            source_record_id: "record:generic-onion".to_owned(),
            source_version: "revision:1".to_owned(),
            evidence_class: ProposalEvidenceClassV1::Published,
            observed_at_epoch_ms: 1_749_000_000_000,
            valid_until_epoch_ms: None,
            market_ref: None,
            license_ref: None,
        });
        request.proposal_sha256 = digest(&request.proposal);
        request.acceptance.proposal_sha256 = request.proposal_sha256.clone();
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Blocked);
        assert!(
            report.issues.iter().any(|issue| {
                issue.code == MealCompilationIssueCodeV1::InvalidAllergenFactScope
            })
        );
    }

    #[test]
    fn food_flow_never_converts_volume_to_mass_without_a_conversion_basis() {
        let mut request = valid_request();
        request.food_flow_edges[0].quantity = qty(250, QuantityUnitV1::Millilitre);
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(report.disposition, MealCompilationDispositionV1::Conflicted);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == MealCompilationIssueCodeV1::FlowQuantityMismatch })
        );
    }

    #[test]
    fn action_mapping_provenance_must_match_the_explicit_resolution() {
        let mut request = valid_request();
        request.task_bindings[0]
            .ontology_binding
            .as_mut()
            .unwrap()
            .mapping
            .provenance_ref = Some("unrelated:ruleset".to_owned());
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(
            report.disposition,
            MealCompilationDispositionV1::Indeterminate
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| { issue.code == MealCompilationIssueCodeV1::MissingActionOntology })
        );
    }

    #[test]
    fn frozen_symthaea_proposal_contract_deserializes_without_inference() {
        let bundle: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/symthaea-meal-proposal-v1.json"))
                .expect("frozen Symthaea proposal fixture must parse");
        assert_eq!(bundle["source"]["repository"], "Luminous-Dynamics/symthaea");
        assert_eq!(bundle["source"]["pull_request"], 12715);
        assert_eq!(
            bundle["source"]["head_sha"],
            "146c70d73315067efca35cf14c9916d340e7fee7"
        );

        let proposal: MealProposalV1 = serde_json::from_value(bundle["proposal"].clone())
            .expect("Symthaea MealProposalV1 wire shape must deserialize in Symtropy");
        assert_eq!(proposal.proposal_id, "fixture:meal-proposal-001");
        assert_eq!(proposal.constraints.currency_code.as_deref(), Some("ZAR"));
        assert_eq!(proposal.review_state, ProposalReviewStateV1::Proposed);
        assert_eq!(
            proposal.ingredients[1].quantity_state,
            IngredientQuantityStateV1::Unknown
        );
        assert!(proposal.ingredients[1].quantity.is_none());
        assert!(
            proposal
                .ingredients
                .iter()
                .all(|ingredient| { ingredient.product_lot_ref.is_none() })
        );
        assert_eq!(
            proposal.uncertainties[0].impact,
            UncertaintyImpactV1::PlanningBlocker
        );
        assert_eq!(proposal.tasks[0].fact_refs, vec!["fixture:recipe"]);
        assert!(proposal.tasks[1].fact_refs.is_empty());
    }

    #[test]
    fn benchmark_manifest_is_versioned_and_has_unique_scenarios() {
        let manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../../../crates/domains/symtropy-kitchen-planner/schemas/fixtures/meal-compilation-v1.json"
        ))
        .unwrap();
        assert_eq!(manifest["status"], "synthetic_only_unexecuted");
        let scenarios = manifest["scenarios"].as_array().unwrap();
        assert!(scenarios.len() >= 16);
        let mut ids = std::collections::BTreeSet::new();
        for scenario in scenarios {
            let id = scenario["id"].as_str().unwrap();
            assert!(ids.insert(id), "duplicate scenario ID: {id}");
            assert!(scenario["expected"].as_str().is_some());
            assert!(scenario["assert"].as_str().is_some());
        }
        assert_eq!(manifest["score_policy"]["false_compiled"], "hard_failure");
        assert_eq!(
            manifest["target_hypotheses"]["false_compiled_adversarial_10000"],
            0
        );
    }

    #[test]
    fn underallocated_ingredient_requires_explicit_residual_or_loss_path() {
        let mut request = valid_request();
        request
            .food_flow_edges
            .retain(|edge| edge.edge_id != "edge:raw-onion-to-chop");
        // The task has no raw input after this removal; the compiler must not
        // infer stock consumption from an output or a planned-yield estimate.
        let report = meal_compiler::compile_meal(&request);
        assert_eq!(
            report.disposition,
            MealCompilationDispositionV1::Indeterminate
        );
        assert!(report.issues.iter().any(|issue| {
            issue.code == MealCompilationIssueCodeV1::IngredientAllocationIncomplete
        }));
    }
}

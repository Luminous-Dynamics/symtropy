// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hierarchical, source-aware action ontology for recipe-to-workcell grounding.
//!
//! Action-core mapping is a planning vocabulary only. It does not define robot
//! trajectories, appliance commands, safety interlocks or food-safety outcomes.
//! Unknown actions remain explicitly unmapped until a human-reviewed mapping is
//! supplied. No lexical-similarity score is treated as execution authority.

use crate::recipe_graph::{RECIPE_ACTION_KINDS_V1, RecipeActionKindV1, RecipeEvidenceClassV1};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const KITCHEN_ACTION_ONTOLOGY_VERSION: u32 = 1;
pub const ACTION_ONTOLOGY_RULESET_REF: &str = "builtin:symtropy-kitchen-action-rules-v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionCoreV1 {
    Cutting,
    Pouring,
    Mixing,
    Preparing,
    PickAndPlace,
    CookAndCool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionGroupV1 {
    Chop,
    Slice,
    Dice,
    Grate,
    Shred,
    Halve,
    Pour,
    Drain,
    Sprinkle,
    Stir,
    Add,
    Combine,
    Whisk,
    Mix,
    Knead,
    Blend,
    Emulsify,
    Wash,
    Peel,
    Trim,
    Pick,
    Place,
    Move,
    Serve,
    Cover,
    Saute,
    Cook,
    Heat,
    Freeze,
    Boil,
    Simmer,
    Steam,
    Bake,
    Roast,
    PressureCook,
    Cool,
    Rest,
}

/// Canonical action-group universe in stable serialized/evaluation order.
pub const ACTION_GROUPS_V1: &[ActionGroupV1] = &[
    ActionGroupV1::Chop,
    ActionGroupV1::Slice,
    ActionGroupV1::Dice,
    ActionGroupV1::Grate,
    ActionGroupV1::Shred,
    ActionGroupV1::Halve,
    ActionGroupV1::Pour,
    ActionGroupV1::Drain,
    ActionGroupV1::Sprinkle,
    ActionGroupV1::Stir,
    ActionGroupV1::Add,
    ActionGroupV1::Combine,
    ActionGroupV1::Whisk,
    ActionGroupV1::Mix,
    ActionGroupV1::Knead,
    ActionGroupV1::Blend,
    ActionGroupV1::Emulsify,
    ActionGroupV1::Wash,
    ActionGroupV1::Peel,
    ActionGroupV1::Trim,
    ActionGroupV1::Pick,
    ActionGroupV1::Place,
    ActionGroupV1::Move,
    ActionGroupV1::Serve,
    ActionGroupV1::Cover,
    ActionGroupV1::Saute,
    ActionGroupV1::Cook,
    ActionGroupV1::Heat,
    ActionGroupV1::Freeze,
    ActionGroupV1::Boil,
    ActionGroupV1::Simmer,
    ActionGroupV1::Steam,
    ActionGroupV1::Bake,
    ActionGroupV1::Roast,
    ActionGroupV1::PressureCook,
    ActionGroupV1::Cool,
    ActionGroupV1::Rest,
];

impl ActionGroupV1 {
    pub const fn core(self) -> ActionCoreV1 {
        match self {
            Self::Chop | Self::Slice | Self::Dice | Self::Grate | Self::Shred | Self::Halve => {
                ActionCoreV1::Cutting
            }
            Self::Pour | Self::Drain | Self::Sprinkle => ActionCoreV1::Pouring,
            Self::Stir
            | Self::Add
            | Self::Combine
            | Self::Whisk
            | Self::Mix
            | Self::Blend
            | Self::Emulsify => ActionCoreV1::Mixing,
            Self::Wash | Self::Peel | Self::Trim | Self::Knead => ActionCoreV1::Preparing,
            Self::Pick | Self::Place | Self::Move | Self::Serve | Self::Cover => {
                ActionCoreV1::PickAndPlace
            }
            Self::Saute
            | Self::Boil
            | Self::Simmer
            | Self::Steam
            | Self::Bake
            | Self::Roast
            | Self::PressureCook
            | Self::Cook
            | Self::Heat
            | Self::Freeze
            | Self::Cool
            | Self::Rest => ActionCoreV1::CookAndCool,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionMappingMethodV1 {
    RuleBased,
    HumanReviewed,
    Unmapped,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionOntologyMappingV1 {
    pub method: ActionMappingMethodV1,
    pub core: Option<ActionCoreV1>,
    pub group: Option<ActionGroupV1>,
    /// The ontology rule or human review artifact that supports this mapping.
    pub provenance_ref: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionParameterUnitV1 {
    Millimetre,
    Centimetre,
    Gram,
    Milligram,
    Millilitre,
    Litre,
    Second,
    Millisecond,
    DegreeCelsius,
    RevolutionsPerMinute,
    Percent,
    Count,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionParameterV1 {
    pub name: String,
    /// Exact fixed-point coefficient; decimal_places gives its scale.
    pub value: i64,
    pub decimal_places: u8,
    pub unit: ActionParameterUnitV1,
    /// Optional pointer to the source or measurement that supplied this value.
    pub source_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionOntologyBindingV1 {
    pub ontology_version: u32,
    pub mapping: ActionOntologyMappingV1,
    /// Empty means no parameter was supplied. The ontology never invents one.
    pub parameters: Vec<ActionParameterV1>,
    /// References to capabilities, not device IDs or executable commands.
    pub tool_capability_refs: Vec<String>,
    /// References to environmental/context preconditions, not actuator states.
    pub precondition_refs: Vec<String>,
    pub evidence_class: RecipeEvidenceClassV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionOntologyViolationCodeV1 {
    UnsupportedOntologyVersion,
    MissingMappingProvenance,
    InvalidUnmappedBinding,
    MissingMappedCoreOrGroup,
    CoreGroupMismatch,
    RuleMappingMismatch,
    MissingHumanReviewProvenance,
    DuplicateParameter,
    InvalidParameterName,
    InvalidParameterScale,
    EmptyParameterSource,
    EmptyCapabilityReference,
    DuplicateCapabilityReference,
    EmptyPreconditionReference,
    DuplicatePreconditionReference,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionOntologyViolationV1 {
    pub code: ActionOntologyViolationCodeV1,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionOntologyValidationReportV1 {
    pub violations: Vec<ActionOntologyViolationV1>,
}

impl ActionOntologyValidationReportV1 {
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }
}

fn violation(
    violations: &mut Vec<ActionOntologyViolationV1>,
    code: ActionOntologyViolationCodeV1,
    detail: impl Into<String>,
) {
    violations.push(ActionOntologyViolationV1 {
        code,
        detail: detail.into(),
    });
}

/// Explicit, deterministic mapping for the built-in recipe action vocabulary.
/// The result for `Other` is intentionally unmapped.
pub fn rule_based_action_mapping(kind: RecipeActionKindV1) -> ActionOntologyMappingV1 {
    let group = match kind {
        RecipeActionKindV1::Wash => Some(ActionGroupV1::Wash),
        RecipeActionKindV1::Peel => Some(ActionGroupV1::Peel),
        RecipeActionKindV1::Trim => Some(ActionGroupV1::Trim),
        RecipeActionKindV1::Chop => Some(ActionGroupV1::Chop),
        RecipeActionKindV1::Slice => Some(ActionGroupV1::Slice),
        RecipeActionKindV1::Dice => Some(ActionGroupV1::Dice),
        RecipeActionKindV1::Grate => Some(ActionGroupV1::Grate),
        RecipeActionKindV1::Shred => Some(ActionGroupV1::Shred),
        RecipeActionKindV1::Halve => Some(ActionGroupV1::Halve),
        RecipeActionKindV1::Pour => Some(ActionGroupV1::Pour),
        RecipeActionKindV1::Drain => Some(ActionGroupV1::Drain),
        RecipeActionKindV1::Sprinkle => Some(ActionGroupV1::Sprinkle),
        RecipeActionKindV1::Stir => Some(ActionGroupV1::Stir),
        RecipeActionKindV1::Add => Some(ActionGroupV1::Add),
        RecipeActionKindV1::Combine => Some(ActionGroupV1::Combine),
        RecipeActionKindV1::Whisk => Some(ActionGroupV1::Whisk),
        RecipeActionKindV1::Mix => Some(ActionGroupV1::Mix),
        RecipeActionKindV1::Knead => Some(ActionGroupV1::Knead),
        RecipeActionKindV1::Blend => Some(ActionGroupV1::Blend),
        RecipeActionKindV1::Emulsify => Some(ActionGroupV1::Emulsify),
        RecipeActionKindV1::Pick => Some(ActionGroupV1::Pick),
        RecipeActionKindV1::Place => Some(ActionGroupV1::Place),
        RecipeActionKindV1::Move => Some(ActionGroupV1::Move),
        RecipeActionKindV1::Cover => Some(ActionGroupV1::Cover),
        RecipeActionKindV1::Saute => Some(ActionGroupV1::Saute),
        RecipeActionKindV1::Cook => Some(ActionGroupV1::Cook),
        RecipeActionKindV1::Heat => Some(ActionGroupV1::Heat),
        RecipeActionKindV1::Freeze => Some(ActionGroupV1::Freeze),
        RecipeActionKindV1::Boil => Some(ActionGroupV1::Boil),
        RecipeActionKindV1::Simmer => Some(ActionGroupV1::Simmer),
        RecipeActionKindV1::Steam => Some(ActionGroupV1::Steam),
        RecipeActionKindV1::Bake => Some(ActionGroupV1::Bake),
        RecipeActionKindV1::Roast => Some(ActionGroupV1::Roast),
        RecipeActionKindV1::PressureCook => Some(ActionGroupV1::PressureCook),
        RecipeActionKindV1::Cool => Some(ActionGroupV1::Cool),
        RecipeActionKindV1::Rest => Some(ActionGroupV1::Rest),
        RecipeActionKindV1::Serve => Some(ActionGroupV1::Serve),
        RecipeActionKindV1::Other => None,
    };
    match group {
        Some(group) => ActionOntologyMappingV1 {
            method: ActionMappingMethodV1::RuleBased,
            core: Some(group.core()),
            group: Some(group),
            provenance_ref: Some(ACTION_ONTOLOGY_RULESET_REF.to_owned()),
        },
        None => ActionOntologyMappingV1 {
            method: ActionMappingMethodV1::Unmapped,
            core: None,
            group: None,
            provenance_ref: None,
        },
    }
}

/// Construct a binding using only deterministic taxonomy evidence. Parameters,
/// tools and preconditions remain empty until separately supplied.
pub fn rule_based_action_binding(
    kind: RecipeActionKindV1,
    evidence_class: RecipeEvidenceClassV1,
) -> ActionOntologyBindingV1 {
    ActionOntologyBindingV1 {
        ontology_version: KITCHEN_ACTION_ONTOLOGY_VERSION,
        mapping: rule_based_action_mapping(kind),
        parameters: Vec::new(),
        tool_capability_refs: Vec::new(),
        precondition_refs: Vec::new(),
        evidence_class,
    }
}

/// Validate mapping provenance, core/group coherence and typed references.
/// Similarity candidates are intentionally not accepted as this V1 schema has
/// no implicit similarity-to-action execution path.
pub fn validate_action_ontology_binding(
    kind: RecipeActionKindV1,
    binding: &ActionOntologyBindingV1,
) -> ActionOntologyValidationReportV1 {
    let mut violations = Vec::new();
    if binding.ontology_version != KITCHEN_ACTION_ONTOLOGY_VERSION {
        violation(
            &mut violations,
            ActionOntologyViolationCodeV1::UnsupportedOntologyVersion,
            format!(
                "unsupported action ontology version {}",
                binding.ontology_version
            ),
        );
    }

    match binding.mapping.method {
        ActionMappingMethodV1::Unmapped => {
            if binding.mapping.core.is_some() || binding.mapping.group.is_some() {
                violation(
                    &mut violations,
                    ActionOntologyViolationCodeV1::InvalidUnmappedBinding,
                    "unmapped action must not carry an executable core/group mapping",
                );
            }
            if binding.mapping.provenance_ref.is_some() {
                violation(
                    &mut violations,
                    ActionOntologyViolationCodeV1::InvalidUnmappedBinding,
                    "unmapped action must not claim a mapping provenance reference",
                );
            }
        }
        ActionMappingMethodV1::RuleBased | ActionMappingMethodV1::HumanReviewed => {
            let (Some(core), Some(group)) = (binding.mapping.core, binding.mapping.group) else {
                violation(
                    &mut violations,
                    ActionOntologyViolationCodeV1::MissingMappedCoreOrGroup,
                    "mapped action requires both a core and an action group",
                );
                return ActionOntologyValidationReportV1 { violations };
            };
            if group.core() != core {
                violation(
                    &mut violations,
                    ActionOntologyViolationCodeV1::CoreGroupMismatch,
                    "action group does not belong to the declared action core",
                );
            }
            if binding
                .mapping
                .provenance_ref
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
            {
                let code = if binding.mapping.method == ActionMappingMethodV1::HumanReviewed {
                    ActionOntologyViolationCodeV1::MissingHumanReviewProvenance
                } else {
                    ActionOntologyViolationCodeV1::MissingMappingProvenance
                };
                violation(
                    &mut violations,
                    code,
                    "mapped action requires a non-empty mapping provenance reference",
                );
            }
            if binding.mapping.method == ActionMappingMethodV1::RuleBased {
                let expected = rule_based_action_mapping(kind);
                if expected.method != ActionMappingMethodV1::RuleBased
                    || expected.core != Some(core)
                    || expected.group != Some(group)
                    || binding.mapping.provenance_ref.as_deref()
                        != expected.provenance_ref.as_deref()
                {
                    violation(
                        &mut violations,
                        ActionOntologyViolationCodeV1::RuleMappingMismatch,
                        "rule-based mapping differs from the deterministic V1 action table",
                    );
                }
            }
        }
    }

    let mut parameter_names = BTreeSet::new();
    for parameter in &binding.parameters {
        if parameter.name.trim().is_empty() {
            violation(
                &mut violations,
                ActionOntologyViolationCodeV1::InvalidParameterName,
                "action parameter name must be non-empty",
            );
        }
        if !parameter_names.insert(parameter.name.as_str()) {
            violation(
                &mut violations,
                ActionOntologyViolationCodeV1::DuplicateParameter,
                format!("duplicate action parameter {}", parameter.name),
            );
        }
        if parameter.decimal_places > 6 {
            violation(
                &mut violations,
                ActionOntologyViolationCodeV1::InvalidParameterScale,
                format!(
                    "parameter {} has more than six decimal places",
                    parameter.name
                ),
            );
        }
        if parameter
            .source_ref
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            violation(
                &mut violations,
                ActionOntologyViolationCodeV1::EmptyParameterSource,
                format!("parameter {} has an empty source reference", parameter.name),
            );
        }
    }

    let mut capability_refs = BTreeSet::new();
    for reference in &binding.tool_capability_refs {
        if reference.trim().is_empty() {
            violation(
                &mut violations,
                ActionOntologyViolationCodeV1::EmptyCapabilityReference,
                "tool capability references must be non-empty",
            );
        }
        if !capability_refs.insert(reference.as_str()) {
            violation(
                &mut violations,
                ActionOntologyViolationCodeV1::DuplicateCapabilityReference,
                format!("duplicate tool capability reference {reference}"),
            );
        }
    }

    let mut precondition_refs = BTreeSet::new();
    for reference in &binding.precondition_refs {
        if reference.trim().is_empty() {
            violation(
                &mut violations,
                ActionOntologyViolationCodeV1::EmptyPreconditionReference,
                "precondition references must be non-empty",
            );
        }
        if !precondition_refs.insert(reference.as_str()) {
            violation(
                &mut violations,
                ActionOntologyViolationCodeV1::DuplicatePreconditionReference,
                format!("duplicate precondition reference {reference}"),
            );
        }
    }

    violations.sort_by(|a, b| (a.code, a.detail.as_str()).cmp(&(b.code, b.detail.as_str())));
    ActionOntologyValidationReportV1 { violations }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionOntologyBenchmarkCaseV1 {
    pub case_id: String,
    pub action_kind: RecipeActionKindV1,
    /// None means an ambiguous or out-of-taxonomy gold action.
    pub gold_group: Option<ActionGroupV1>,
    pub prediction: ActionOntologyBindingV1,
    /// Corpus occurrence count or a declared sampling weight.
    pub occurrence_weight: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionOntologyBenchmarkErrorCodeV1 {
    EmptyCaseSet,
    EmptyCaseId,
    DuplicateCaseId,
    ZeroOccurrenceWeight,
    InvalidPrediction,
    AggregateOverflow,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionOntologyBenchmarkErrorV1 {
    pub code: ActionOntologyBenchmarkErrorCodeV1,
    pub case_id: Option<String>,
    pub details: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionGroupEvaluationCountsV1 {
    pub group: ActionGroupV1,
    pub gold_occurrences: u64,
    pub predicted_occurrences: u64,
    pub true_positive_occurrences: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionOntologyBenchmarkSummaryV1 {
    pub case_count: u64,
    pub total_occurrence_weight: u64,
    pub mapped_occurrence_weight: u64,
    pub gold_mapped_occurrence_weight: u64,
    pub correct_mapped_occurrence_weight: u64,
    pub abstained_occurrence_weight: u64,
    pub correctly_abstained_occurrence_weight: u64,
    pub groups: Vec<ActionGroupEvaluationCountsV1>,
}

impl ActionOntologyBenchmarkSummaryV1 {
    /// Fraction = mapped_occurrence_weight / total_occurrence_weight.
    pub fn weighted_core_coverage(&self) -> (u64, u64) {
        (self.mapped_occurrence_weight, self.total_occurrence_weight)
    }

    /// Micro precision = correct mappings / all predicted mappings.
    pub fn micro_group_precision(&self) -> (u64, u64) {
        (
            self.correct_mapped_occurrence_weight,
            self.mapped_occurrence_weight,
        )
    }

    /// Micro recall = correct mappings / all gold-mapped occurrences.
    pub fn micro_group_recall(&self) -> (u64, u64) {
        (
            self.correct_mapped_occurrence_weight,
            self.gold_mapped_occurrence_weight,
        )
    }

    /// Abstention precision = correct abstentions / all predicted abstentions.
    pub fn abstention_precision(&self) -> (u64, u64) {
        (
            self.correctly_abstained_occurrence_weight,
            self.abstained_occurrence_weight,
        )
    }
}

fn checked_accumulate(
    target: &mut u64,
    increment: u64,
    case_id: &str,
) -> Result<(), ActionOntologyBenchmarkErrorV1> {
    let Some(value) = target.checked_add(increment) else {
        return Err(ActionOntologyBenchmarkErrorV1 {
            code: ActionOntologyBenchmarkErrorCodeV1::AggregateOverflow,
            case_id: Some(case_id.to_owned()),
            details: vec!["weighted benchmark count overflowed u64".to_owned()],
        });
    };
    *target = value;
    Ok(())
}

/// Evaluate action-core/group mappings against frozen adjudicated labels.
///
/// All metrics retain exact integer numerators/denominators. Ambiguous or
/// out-of-taxonomy gold labels use None; an unsupported guess is a false
/// positive. Invalid mappings fail the evaluation instead of being scored.
pub fn evaluate_action_ontology_benchmark(
    cases: &[ActionOntologyBenchmarkCaseV1],
) -> Result<ActionOntologyBenchmarkSummaryV1, ActionOntologyBenchmarkErrorV1> {
    if cases.is_empty() {
        return Err(ActionOntologyBenchmarkErrorV1 {
            code: ActionOntologyBenchmarkErrorCodeV1::EmptyCaseSet,
            case_id: None,
            details: vec!["benchmark case set must not be empty".to_owned()],
        });
    }

    let mut seen_ids = BTreeSet::new();
    let mut summary = ActionOntologyBenchmarkSummaryV1 {
        case_count: 0,
        total_occurrence_weight: 0,
        mapped_occurrence_weight: 0,
        gold_mapped_occurrence_weight: 0,
        correct_mapped_occurrence_weight: 0,
        abstained_occurrence_weight: 0,
        correctly_abstained_occurrence_weight: 0,
        groups: ACTION_GROUPS_V1
            .iter()
            .copied()
            .map(|group| ActionGroupEvaluationCountsV1 {
                group,
                gold_occurrences: 0,
                predicted_occurrences: 0,
                true_positive_occurrences: 0,
            })
            .collect(),
    };

    for case in cases {
        if case.case_id.trim().is_empty() {
            return Err(ActionOntologyBenchmarkErrorV1 {
                code: ActionOntologyBenchmarkErrorCodeV1::EmptyCaseId,
                case_id: Some(case.case_id.clone()),
                details: vec!["benchmark case_id must be non-empty".to_owned()],
            });
        }
        if !seen_ids.insert(case.case_id.as_str()) {
            return Err(ActionOntologyBenchmarkErrorV1 {
                code: ActionOntologyBenchmarkErrorCodeV1::DuplicateCaseId,
                case_id: Some(case.case_id.clone()),
                details: vec!["benchmark case_id must be unique".to_owned()],
            });
        }
        if case.occurrence_weight == 0 {
            return Err(ActionOntologyBenchmarkErrorV1 {
                code: ActionOntologyBenchmarkErrorCodeV1::ZeroOccurrenceWeight,
                case_id: Some(case.case_id.clone()),
                details: vec!["occurrence_weight must be positive".to_owned()],
            });
        }

        let validation = validate_action_ontology_binding(case.action_kind, &case.prediction);
        if !validation.is_valid() {
            return Err(ActionOntologyBenchmarkErrorV1 {
                code: ActionOntologyBenchmarkErrorCodeV1::InvalidPrediction,
                case_id: Some(case.case_id.clone()),
                details: validation
                    .violations
                    .into_iter()
                    .map(|entry| format!("{:?}: {}", entry.code, entry.detail))
                    .collect(),
            });
        }

        let predicted_group = case.prediction.mapping.group;
        checked_accumulate(&mut summary.case_count, 1, &case.case_id)?;
        checked_accumulate(
            &mut summary.total_occurrence_weight,
            case.occurrence_weight,
            &case.case_id,
        )?;

        if let Some(group) = case.gold_group {
            checked_accumulate(
                &mut summary.gold_mapped_occurrence_weight,
                case.occurrence_weight,
                &case.case_id,
            )?;
            let counts = summary
                .groups
                .iter_mut()
                .find(|counts| counts.group == group)
                .expect("every action group has a stable evaluation entry");
            checked_accumulate(
                &mut counts.gold_occurrences,
                case.occurrence_weight,
                &case.case_id,
            )?;
        }

        if let Some(group) = predicted_group {
            checked_accumulate(
                &mut summary.mapped_occurrence_weight,
                case.occurrence_weight,
                &case.case_id,
            )?;
            let counts = summary.groups.iter_mut().find(|counts| counts.group == group)
                .expect("every action group has a stable evaluation entry");
            checked_accumulate(
                &mut counts.predicted_occurrences,
                case.occurrence_weight,
                &case.case_id,
            )?;
        } else {
            checked_accumulate(
                &mut summary.abstained_occurrence_weight,
                case.occurrence_weight,
                &case.case_id,
            )?;
        }

        if predicted_group.is_some() && predicted_group == case.gold_group {
            checked_accumulate(
                &mut summary.correct_mapped_occurrence_weight,
                case.occurrence_weight,
                &case.case_id,
            )?;
            let group = predicted_group.expect("checked as Some");
            let counts = summary.groups.iter_mut().find(|counts| counts.group == group)
                .expect("every action group has a stable evaluation entry");
            checked_accumulate(
                &mut counts.true_positive_occurrences,
                case.occurrence_weight,
                &case.case_id,
            )?;
        } else if predicted_group.is_none() && case.gold_group.is_none() {
            checked_accumulate(
                &mut summary.correctly_abstained_occurrence_weight,
                case.occurrence_weight,
                &case.case_id,
            )?;
        }
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct ActionMappingFixtureBundle {
        mappings: Vec<ActionMappingFixture>,
    }

    #[derive(Deserialize)]
    struct ActionMappingFixture {
        action_kind: RecipeActionKindV1,
        expected_core: Option<ActionCoreV1>,
        expected_group: Option<ActionGroupV1>,
        expected_method: ActionMappingMethodV1,
    }

    #[test]
    fn action_mapping_fixtures_match_deterministic_rules() {
        let bundle: ActionMappingFixtureBundle = serde_json::from_str(include_str!(
            "../schemas/fixtures/action-ontology-v1.json"
        ))
        .expect("action ontology fixture bundle must deserialize");
        for fixture in bundle.mappings {
            let mapping = rule_based_action_mapping(fixture.action_kind);
            assert_eq!(mapping.core, fixture.expected_core);
            assert_eq!(mapping.group, fixture.expected_group);
            assert_eq!(mapping.method, fixture.expected_method);
        }
    }

    #[test]
    fn every_recipe_action_kind_has_exactly_one_fixture_mapping() {
        let bundle: ActionMappingFixtureBundle = serde_json::from_str(include_str!(
            "../schemas/fixtures/action-ontology-v1.json"
        ))
        .expect("action ontology fixture bundle must deserialize");
        let fixture_kinds: Vec<_> = bundle
            .mappings
            .iter()
            .map(|fixture| fixture.action_kind)
            .collect();

        assert_eq!(
            fixture_kinds.len(),
            RECIPE_ACTION_KINDS_V1.len(),
            "fixture mapping count must equal the canonical recipe action vocabulary"
        );
        for kind in RECIPE_ACTION_KINDS_V1 {
            assert_eq!(
                fixture_kinds.iter().filter(|candidate| *candidate == kind).count(),
                1,
                "recipe action kind {kind:?} must have exactly one fixture mapping"
            );
        }
    }

    #[test]
    fn unmapped_action_cannot_carry_a_core_or_group() {
        let mut binding = rule_based_action_binding(
            RecipeActionKindV1::Other,
            RecipeEvidenceClassV1::Modelled,
        );
        binding.mapping.core = Some(ActionCoreV1::Preparing);
        let report = validate_action_ontology_binding(RecipeActionKindV1::Other, &binding);
        assert!(report.violations.iter().any(|entry| {
            entry.code == ActionOntologyViolationCodeV1::InvalidUnmappedBinding
        }));
    }

    #[test]
    fn duplicate_action_parameters_are_rejected() {
        let mut binding = rule_based_action_binding(
            RecipeActionKindV1::Chop,
            RecipeEvidenceClassV1::Declared,
        );
        let parameter = ActionParameterV1 {
            name: "cut_width".to_owned(),
            value: 5,
            decimal_places: 0,
            unit: ActionParameterUnitV1::Millimetre,
            source_ref: Some("fixture:recipe-measurement".to_owned()),
        };
        binding.parameters = vec![parameter.clone(), parameter];
        let report = validate_action_ontology_binding(RecipeActionKindV1::Chop, &binding);
        assert!(report.violations.iter().any(|entry| {
            entry.code == ActionOntologyViolationCodeV1::DuplicateParameter
        }));
    }

    #[derive(Deserialize)]
    struct ActionOntologyEvaluationFixtureBundle {
        cases: Vec<ActionOntologyBenchmarkCaseV1>,
        expected: ActionOntologyEvaluationExpected,
    }

    #[derive(Deserialize)]
    struct ActionOntologyEvaluationExpected {
        case_count: u64,
        total_occurrence_weight: u64,
        weighted_core_coverage: [u64; 2],
        micro_group_precision: [u64; 2],
        micro_group_recall: [u64; 2],
        abstention_precision: [u64; 2],
    }

    #[test]
    fn weighted_action_ontology_json_fixture_matches_rust_evaluator() {
        let bundle: ActionOntologyEvaluationFixtureBundle = serde_json::from_str(include_str!(
            "../schemas/fixtures/action-ontology-evaluation-v1.json"
        ))
        .expect("action ontology evaluation fixture must deserialize");
        let report = evaluate_action_ontology_benchmark(&bundle.cases).unwrap();
        assert_eq!(report.case_count, bundle.expected.case_count);
        assert_eq!(report.total_occurrence_weight, bundle.expected.total_occurrence_weight);
        assert_eq!(report.weighted_core_coverage(), bundle.expected.weighted_core_coverage);
        assert_eq!(report.micro_group_precision(), bundle.expected.micro_group_precision);
        assert_eq!(report.micro_group_recall(), bundle.expected.micro_group_recall);
        assert_eq!(report.abstention_precision(), bundle.expected.abstention_precision);
    }

    #[test]
    fn benchmark_reports_weighted_coverage_precision_recall_and_abstention() {
        let cases = vec![
            ActionOntologyBenchmarkCaseV1 {
                case_id: "chop-correct".to_owned(),
                action_kind: RecipeActionKindV1::Chop,
                gold_group: Some(ActionGroupV1::Chop),
                prediction: rule_based_action_binding(RecipeActionKindV1::Chop, RecipeEvidenceClassV1::Declared),
                occurrence_weight: 3,
            },
            ActionOntologyBenchmarkCaseV1 {
                case_id: "other-abstained".to_owned(),
                action_kind: RecipeActionKindV1::Other,
                gold_group: None,
                prediction: rule_based_action_binding(RecipeActionKindV1::Other, RecipeEvidenceClassV1::Modelled),
                occurrence_weight: 2,
            },
            ActionOntologyBenchmarkCaseV1 {
                case_id: "saute-wrong-group".to_owned(),
                action_kind: RecipeActionKindV1::Saute,
                gold_group: Some(ActionGroupV1::Boil),
                prediction: rule_based_action_binding(RecipeActionKindV1::Saute, RecipeEvidenceClassV1::Declared),
                occurrence_weight: 1,
            },
        ];
        let report = evaluate_action_ontology_benchmark(&cases).unwrap();
        assert_eq!(report.weighted_core_coverage(), (4, 6));
        assert_eq!(report.micro_group_precision(), (3, 4));
        assert_eq!(report.micro_group_recall(), (3, 4));
        assert_eq!(report.abstention_precision(), (2, 2));
        assert_eq!(report.case_count, 3);
    }

    #[test]
    fn benchmark_rejects_duplicate_case_ids_and_zero_weights() {
        let mut case = ActionOntologyBenchmarkCaseV1 {
            case_id: "same".to_owned(),
            action_kind: RecipeActionKindV1::Chop,
            gold_group: Some(ActionGroupV1::Chop),
            prediction: rule_based_action_binding(RecipeActionKindV1::Chop, RecipeEvidenceClassV1::Declared),
            occurrence_weight: 1,
        };
        let duplicate = evaluate_action_ontology_benchmark(&[case.clone(), case.clone()]);
        assert_eq!(duplicate.unwrap_err().code, ActionOntologyBenchmarkErrorCodeV1::DuplicateCaseId);
        case.occurrence_weight = 0;
        let zero = evaluate_action_ontology_benchmark(&[case]);
        assert_eq!(zero.unwrap_err().code, ActionOntologyBenchmarkErrorCodeV1::ZeroOccurrenceWeight);
    }

    #[test]
    fn benchmark_rejects_invalid_ontology_predictions() {
        let mut prediction = rule_based_action_binding(RecipeActionKindV1::Chop, RecipeEvidenceClassV1::Declared);
        prediction.mapping.group = Some(ActionGroupV1::Dice);
        let case = ActionOntologyBenchmarkCaseV1 {
            case_id: "invalid-prediction".to_owned(),
            action_kind: RecipeActionKindV1::Chop,
            gold_group: Some(ActionGroupV1::Chop),
            prediction,
            occurrence_weight: 1,
        };
        let error = evaluate_action_ontology_benchmark(&[case]).unwrap_err();
        assert_eq!(error.code, ActionOntologyBenchmarkErrorCodeV1::InvalidPrediction);
    }

    #[test]
    fn other_action_stays_unmapped_without_human_review() {
        let binding = rule_based_action_binding(
            RecipeActionKindV1::Other,
            RecipeEvidenceClassV1::Modelled,
        );
        assert!(validate_action_ontology_binding(RecipeActionKindV1::Other, &binding).is_valid());
        assert_eq!(binding.mapping.core, None);
        assert_eq!(binding.mapping.group, None);
    }

    #[test]
    fn human_reviewed_mapping_can_bind_an_other_action_with_provenance() {
        let mut binding = rule_based_action_binding(
            RecipeActionKindV1::Other,
            RecipeEvidenceClassV1::Declared,
        );
        binding.mapping = ActionOntologyMappingV1 {
            method: ActionMappingMethodV1::HumanReviewed,
            core: Some(ActionCoreV1::Preparing),
            group: Some(ActionGroupV1::Trim),
            provenance_ref: Some("review:human-action-mapping-001".to_owned()),
        };
        assert!(validate_action_ontology_binding(RecipeActionKindV1::Other, &binding).is_valid());
    }

    #[test]
    fn a_rule_mapping_cannot_silently_change_its_group() {
        let mut binding = rule_based_action_binding(
            RecipeActionKindV1::Chop,
            RecipeEvidenceClassV1::Declared,
        );
        binding.mapping.group = Some(ActionGroupV1::Dice);
        binding.mapping.core = Some(ActionCoreV1::Cutting);
        assert!(validate_action_ontology_binding(RecipeActionKindV1::Chop, &binding)
            .violations
            .iter()
            .any(|entry| entry.code == ActionOntologyViolationCodeV1::RuleMappingMismatch));
    }
}

// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hierarchical, source-aware action ontology for recipe-to-workcell grounding.
//!
//! Action-core mapping is a planning vocabulary only. It does not define robot
//! trajectories, appliance commands, safety interlocks or food-safety outcomes.
//! Unknown actions remain explicitly unmapped until a human-reviewed mapping is
//! supplied. No lexical-similarity score is treated as execution authority.

use crate::recipe_graph::{RecipeActionKindV1, RecipeEvidenceClassV1};
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
    Pour,
    Sprinkle,
    Stir,
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
    Saute,
    Boil,
    Simmer,
    Steam,
    Bake,
    Roast,
    PressureCook,
    Cool,
    Rest,
}

impl ActionGroupV1 {
    pub const fn core(self) -> ActionCoreV1 {
        match self {
            Self::Chop | Self::Slice | Self::Dice | Self::Grate | Self::Shred => {
                ActionCoreV1::Cutting
            }
            Self::Pour | Self::Sprinkle => ActionCoreV1::Pouring,
            Self::Stir | Self::Mix | Self::Knead | Self::Blend | Self::Emulsify => {
                ActionCoreV1::Mixing
            }
            Self::Wash | Self::Peel | Self::Trim => ActionCoreV1::Preparing,
            Self::Pick | Self::Place | Self::Move | Self::Serve => {
                ActionCoreV1::PickAndPlace
            }
            Self::Saute
            | Self::Boil
            | Self::Simmer
            | Self::Steam
            | Self::Bake
            | Self::Roast
            | Self::PressureCook
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
        RecipeActionKindV1::Pour => Some(ActionGroupV1::Pour),
        RecipeActionKindV1::Sprinkle => Some(ActionGroupV1::Sprinkle),
        RecipeActionKindV1::Stir => Some(ActionGroupV1::Stir),
        RecipeActionKindV1::Mix => Some(ActionGroupV1::Mix),
        RecipeActionKindV1::Knead => Some(ActionGroupV1::Knead),
        RecipeActionKindV1::Blend => Some(ActionGroupV1::Blend),
        RecipeActionKindV1::Emulsify => Some(ActionGroupV1::Emulsify),
        RecipeActionKindV1::Pick => Some(ActionGroupV1::Pick),
        RecipeActionKindV1::Place => Some(ActionGroupV1::Place),
        RecipeActionKindV1::Move => Some(ActionGroupV1::Move),
        RecipeActionKindV1::Saute => Some(ActionGroupV1::Saute),
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
            format!("unsupported action ontology version {}", binding.ontology_version),
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
            if binding.mapping.provenance_ref.as_deref().is_none_or(|value| value.trim().is_empty()) {
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
                format!("parameter {} has more than six decimal places", parameter.name),
            );
        }
        if parameter.source_ref.as_deref().is_some_and(|value| value.trim().is_empty()) {
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

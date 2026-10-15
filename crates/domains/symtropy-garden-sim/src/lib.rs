// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic exact-accounting checker for non-clinical garden infrastructure.
//!
//! The checker validates one explicitly declared resource-balance boundary. It is
//! not a coupled physical simulator, farm-yield estimator, water-quality evaluator,
//! or manufacturing qualification tool. A consistent ledger means only that the
//! supplied integer accounting closes within the declared tolerance and boundary.

use serde::{Deserialize, Deserializer, Serialize, Serializer};


/// Serialize exact integers as decimal strings so JSON consumers that use
/// IEEE-754 numbers (for example, JavaScript) do not silently round quantities.
fn serialize_u64_as_string<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_u64_from_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    raw.parse::<u64>().map_err(serde::de::Error::custom)
}

fn serialize_i128_as_string<S>(value: &i128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_i128_from_string<'de, D>(deserializer: D) -> Result<i128, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    raw.parse::<i128>().map_err(serde::de::Error::custom)
}

fn serialize_u128_as_string<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

fn deserialize_u128_from_string<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    raw.parse::<u128>().map_err(serde::de::Error::custom)
}

pub const GARDEN_RESOURCE_SCHEMA_VERSION: u16 = 1;

/// Each resource kind fixes the unit for every quantity in its balance record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    /// Integer millilitres.
    WaterMillilitres,
    /// Integer watt-hours.
    ElectricalEnergyWattHours,
    /// Integer grams.
    MaterialGrams,
}

/// Source class is provenance metadata, not proof that the source is authentic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    SyntheticAssumption,
    ExternalObservationReference,
    QualifiedDispositionReference,
    SimulationDerived,
}

/// One interval balance. Flows/loss/tolerance use the non-negative unit selected
/// by resource_kind. The storage delta is signed because a store can shrink.
/// This checker does not convert units or authenticate source evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBalanceInputV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub resource_kind: ResourceKind,
    #[serde(serialize_with = "serialize_u64_as_string", deserialize_with = "deserialize_u64_from_string")]
    pub inflow: u64,
    #[serde(serialize_with = "serialize_u64_as_string", deserialize_with = "deserialize_u64_from_string")]
    pub generated_or_added: u64,
    #[serde(serialize_with = "serialize_u64_as_string", deserialize_with = "deserialize_u64_from_string")]
    pub outflow: u64,
    #[serde(serialize_with = "serialize_u64_as_string", deserialize_with = "deserialize_u64_from_string")]
    pub declared_loss: u64,
    #[serde(serialize_with = "serialize_i128_as_string", deserialize_with = "deserialize_i128_from_string")]
    pub observed_storage_delta: i128,
    #[serde(serialize_with = "serialize_u64_as_string", deserialize_with = "deserialize_u64_from_string")]
    pub tolerance: u64,
    pub evidence_class: EvidenceClass,
    pub source_ref: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BalanceBlockerCode {
    UnsupportedSchema,
    EmptyScenarioId,
    MissingSourceReference,
    ObservedBalanceMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceBlocker {
    pub code: BalanceBlockerCode,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BalanceDisposition {
    /// Integer accounting is consistent for this declared boundary only.
    AccountingConsistentForDeclaredBoundary,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBalanceDecisionV1 {
    pub scenario_id: String,
    pub resource_kind: ResourceKind,
    pub disposition: BalanceDisposition,
    #[serde(serialize_with = "serialize_i128_as_string", deserialize_with = "deserialize_i128_from_string")]
    pub expected_storage_delta: i128,
    #[serde(serialize_with = "serialize_u128_as_string", deserialize_with = "deserialize_u128_from_string")]
    pub absolute_error: u128,
    pub blockers: Vec<BalanceBlocker>,
}

/// Independently recompute:
/// inflow + generated_or_added - outflow - declared_loss = storage_delta.
///
/// Wide integer arithmetic and abs_diff avoid floating-point rounding. For the
/// admitted u64 flows, the expected delta fits in i128; error magnitude is u128.
/// A result never proves model fidelity, resource safety, horticultural suitability,
/// medicine quality, or real-world service sufficiency.
pub fn evaluate_resource_balance(input: &ResourceBalanceInputV1) -> ResourceBalanceDecisionV1 {
    let mut blockers = Vec::new();

    if input.schema_version != GARDEN_RESOURCE_SCHEMA_VERSION {
        push_blocker(
            &mut blockers,
            BalanceBlockerCode::UnsupportedSchema,
            "unsupported resource-balance schema version",
        );
    }
    if input.scenario_id.trim().is_empty() {
        push_blocker(
            &mut blockers,
            BalanceBlockerCode::EmptyScenarioId,
            "scenario identity is empty",
        );
    }
    if input.source_ref.trim().is_empty() {
        push_blocker(
            &mut blockers,
            BalanceBlockerCode::MissingSourceReference,
            "a source/assumption reference is required",
        );
    }

    let expected = i128::from(input.inflow) + i128::from(input.generated_or_added)
        - i128::from(input.outflow)
        - i128::from(input.declared_loss);
    let error = expected.abs_diff(input.observed_storage_delta);

    if error > u128::from(input.tolerance) {
        push_blocker(
            &mut blockers,
            BalanceBlockerCode::ObservedBalanceMismatch,
            "observed storage delta exceeds the declared integer balance tolerance",
        );
    }

    let disposition = if blockers.is_empty() {
        BalanceDisposition::AccountingConsistentForDeclaredBoundary
    } else {
        BalanceDisposition::Blocked
    };

    ResourceBalanceDecisionV1 {
        scenario_id: input.scenario_id.clone(),
        resource_kind: input.resource_kind,
        disposition,
        expected_storage_delta: expected,
        absolute_error: error,
        blockers,
    }
}

fn push_blocker(
    blockers: &mut Vec<BalanceBlocker>,
    code: BalanceBlockerCode,
    detail: &str,
) {
    blockers.push(BalanceBlocker {
        code,
        detail: detail.to_owned(),
    });
}


/// Nutrient contribution preserves the exact food state and its source lineage.
/// Units name the base nutrient unit; value_scale provides decimal precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientUnit {
    Micrograms,
    Milligrams,
    Grams,
    Kilocalories,
    Kilojoules,
}

/// Provenance class for a composition value. This does not authenticate its source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientCompositionEvidenceClass {
    LaboratoryAnalysis,
    NationalFoodCompositionTable,
    PublishedLiterature,
    FoodLabel,
    ReviewedEstimate,
    SyntheticFixture,
}

/// One nutrient value for an exact food record and food/preparation state.
/// amount_per_100g_scaled / value_scale is the nutrient amount per 100 g in unit.
/// None means missing data, never a zero nutrient value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientCompositionRecordV1 {
    pub schema_version: u16,
    pub food_record_id: String,
    pub food_state_id: String,
    pub nutrient_id: String,
    pub nutrient_definition_ref: String,
    pub unit: NutrientUnit,
    #[serde(
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string"
    )]
    pub amount_per_100g_scaled: Option<u64>,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub value_scale: u64,
    pub evidence_class: NutrientCompositionEvidenceClass,
    pub source_ref: String,
}

/// Mass that actually arrives for the declared scenario. Harvested biomass is not
/// accepted as a substitute: the caller must account for edible fraction, losses,
/// and allocation before supplying edible_delivered_grams.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdibleFoodDeliveryV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub food_record_id: String,
    pub food_state_id: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub edible_delivered_grams: u64,
    pub delivery_source_ref: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientContributionBlockerCode {
    UnsupportedCompositionSchema,
    UnsupportedDeliverySchema,
    EmptyScenarioId,
    EmptyFoodRecordId,
    EmptyFoodStateId,
    EmptyNutrientId,
    EmptyNutrientDefinitionReference,
    FoodRecordMismatch,
    FoodStateMismatch,
    ZeroValueScale,
    MissingCompositionSource,
    MissingDeliverySource,
    MissingCompositionValue,
}

/// Stable validation blocker. Ordering follows the evaluator's fixed validation sequence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientContributionBlocker {
    pub code: NutrientContributionBlockerCode,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientContributionDisposition {
    /// Exact contribution is available for this declared food state and delivery mass.
    CalculatedForDeclaredDelivery,
    /// Composition value is absent; no numerical contribution is inferred.
    NotAssessable,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientContributionDecisionV1 {
    pub scenario_id: String,
    pub food_record_id: String,
    pub food_state_id: String,
    pub nutrient_id: String,
    pub nutrient_definition_ref: String,
    pub unit: NutrientUnit,
    pub value_scale: Option<u64>,
    /// Exact rational contribution in the declared nutrient unit:
    /// numerator / denominator. No float conversion or rounding is performed.
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub contribution_numerator: Option<u128>,
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub contribution_denominator: Option<u128>,
    pub composition_source_ref: String,
    pub delivery_source_ref: String,
    pub disposition: NutrientContributionDisposition,
    pub blockers: Vec<NutrientContributionBlocker>,
}

/// Compute one nutrient contribution without inferring edible yield or source truth.
///
/// If composition amount A is expressed as A_scaled / value_scale per 100 g and
/// E grams were delivered, the exact amount is
/// (A_scaled * E) / (100 * value_scale) in the declared NutrientUnit.
/// u64 * u64 fits in u128, so the numerator is exact. Source refs are preserved,
/// not authenticated; this function cannot certify diet adequacy or food safety.
pub fn evaluate_nutrient_contribution(
    composition: &NutrientCompositionRecordV1,
    delivery: &EdibleFoodDeliveryV1,
) -> NutrientContributionDecisionV1 {
    let mut blockers = Vec::new();

    if composition.schema_version != GARDEN_RESOURCE_SCHEMA_VERSION {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::UnsupportedCompositionSchema,
            "unsupported nutrient-composition schema version",
        );
    }
    if delivery.schema_version != GARDEN_RESOURCE_SCHEMA_VERSION {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::UnsupportedDeliverySchema,
            "unsupported edible-delivery schema version",
        );
    }
    if delivery.scenario_id.trim().is_empty() {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::EmptyScenarioId,
            "scenario identity is empty",
        );
    }
    if composition.food_record_id.trim().is_empty() || delivery.food_record_id.trim().is_empty() {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::EmptyFoodRecordId,
            "food record identity is empty",
        );
    }
    if composition.food_state_id.trim().is_empty() || delivery.food_state_id.trim().is_empty() {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::EmptyFoodStateId,
            "food/preparation state identity is empty",
        );
    }
    if composition.nutrient_id.trim().is_empty() {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::EmptyNutrientId,
            "nutrient identity is empty",
        );
    }
    if composition.nutrient_definition_ref.trim().is_empty() {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::EmptyNutrientDefinitionReference,
            "canonical nutrient-definition reference is missing",
        );
    }
    if composition.food_record_id != delivery.food_record_id {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::FoodRecordMismatch,
            "delivery and nutrient record refer to different foods",
        );
    }
    if composition.food_state_id != delivery.food_state_id {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::FoodStateMismatch,
            "delivery and nutrient record use different food/preparation states",
        );
    }
    if composition.value_scale == 0 {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::ZeroValueScale,
            "composition value scale must be greater than zero",
        );
    }
    if composition.source_ref.trim().is_empty() {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::MissingCompositionSource,
            "composition source reference is required",
        );
    }
    if delivery.delivery_source_ref.trim().is_empty() {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::MissingDeliverySource,
            "edible-delivery evidence reference is required",
        );
    }
    if composition.amount_per_100g_scaled.is_none() {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::MissingCompositionValue,
            "nutrient composition is missing; missing does not mean zero",
        );
    }

    let has_hard_blocker = blockers
        .iter()
        .any(|item| item.code != NutrientContributionBlockerCode::MissingCompositionValue);
    let assessable = composition.amount_per_100g_scaled.is_some();
    let disposition = if has_hard_blocker {
        NutrientContributionDisposition::Blocked
    } else if !assessable {
        NutrientContributionDisposition::NotAssessable
    } else {
        NutrientContributionDisposition::CalculatedForDeclaredDelivery
    };

    let contribution_numerator = if disposition
        == NutrientContributionDisposition::CalculatedForDeclaredDelivery
    {
        composition.amount_per_100g_scaled.map(|amount| {
            u128::from(amount) * u128::from(delivery.edible_delivered_grams)
        })
    } else {
        None
    };
    let contribution_denominator = if disposition
        == NutrientContributionDisposition::CalculatedForDeclaredDelivery
    {
        Some(100 * u128::from(composition.value_scale))
    } else {
        None
    };

    NutrientContributionDecisionV1 {
        scenario_id: delivery.scenario_id.clone(),
        food_record_id: composition.food_record_id.clone(),
        food_state_id: composition.food_state_id.clone(),
        nutrient_id: composition.nutrient_id.clone(),
        nutrient_definition_ref: composition.nutrient_definition_ref.clone(),
        unit: composition.unit,
        value_scale: (composition.value_scale != 0).then_some(composition.value_scale),
        contribution_numerator,
        contribution_denominator,
        composition_source_ref: composition.source_ref.clone(),
        delivery_source_ref: delivery.delivery_source_ref.clone(),
        disposition,
        blockers,
    }
}

fn push_nutrient_blocker(
    blockers: &mut Vec<NutrientContributionBlocker>,
    code: NutrientContributionBlockerCode,
    detail: &str,
) {
    blockers.push(NutrientContributionBlocker {
        code,
        detail: detail.to_owned(),
    });
}

fn serialize_option_u64_as_string<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_some(&value.to_string()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_option_u64_from_string<'de, D>(
    deserializer: D,
) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|raw| raw.parse::<u64>().map_err(serde::de::Error::custom))
        .transpose()
}

fn serialize_option_u128_as_string<S>(
    value: &Option<u128>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_some(&value.to_string()),
        None => serializer.serialize_none(),
    }
}

fn deserialize_option_u128_from_string<'de, D>(
    deserializer: D,
) -> Result<Option<u128>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|raw| raw.parse::<u128>().map_err(serde::de::Error::custom))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_water_input() -> ResourceBalanceInputV1 {
        ResourceBalanceInputV1 {
            schema_version: GARDEN_RESOURCE_SCHEMA_VERSION,
            scenario_id: "SYNTH-GARDEN-WATER-001".to_owned(),
            resource_kind: ResourceKind::WaterMillilitres,
            inflow: 10_000,
            generated_or_added: 0,
            outflow: 4_000,
            declared_loss: 1_000,
            observed_storage_delta: 5_000,
            tolerance: 10,
            evidence_class: EvidenceClass::SyntheticAssumption,
            source_ref: "fixture:SYNTH-GARDEN-WATER-001".to_owned(),
        }
    }

    #[test]
    fn balanced_water_interval_is_accounting_consistent_only() {
        let decision = evaluate_resource_balance(&valid_water_input());
        assert_eq!(
            decision.disposition,
            BalanceDisposition::AccountingConsistentForDeclaredBoundary
        );
        assert_eq!(decision.expected_storage_delta, 5_000);
        assert_eq!(decision.absolute_error, 0);
    }

    #[test]
    fn mismatched_observation_is_blocked() {
        let mut input = valid_water_input();
        input.observed_storage_delta = 4_500;
        let decision = evaluate_resource_balance(&input);
        assert_eq!(decision.disposition, BalanceDisposition::Blocked);
        assert_eq!(decision.absolute_error, 500);
        assert!(
            decision
                .blockers
                .iter()
                .any(|item| { item.code == BalanceBlockerCode::ObservedBalanceMismatch })
        );
    }

    #[test]
    fn tolerance_boundary_is_inclusive_and_exact() {
        let mut input = valid_water_input();
        input.observed_storage_delta = 4_990;
        input.tolerance = 10;
        assert_eq!(
            evaluate_resource_balance(&input).disposition,
            BalanceDisposition::AccountingConsistentForDeclaredBoundary
        );

        input.tolerance = 9;
        assert_eq!(
            evaluate_resource_balance(&input).disposition,
            BalanceDisposition::Blocked
        );
    }

    #[test]
    fn missing_source_reference_is_blocked() {
        let mut input = valid_water_input();
        input.source_ref.clear();
        let decision = evaluate_resource_balance(&input);
        assert!(
            decision
                .blockers
                .iter()
                .any(|item| { item.code == BalanceBlockerCode::MissingSourceReference })
        );
    }

    #[test]
    fn unsupported_schema_is_blocked() {
        let mut input = valid_water_input();
        input.schema_version = GARDEN_RESOURCE_SCHEMA_VERSION + 1;
        let decision = evaluate_resource_balance(&input);
        assert!(
            decision
                .blockers
                .iter()
                .any(|item| { item.code == BalanceBlockerCode::UnsupportedSchema })
        );
    }

    #[test]
    fn empty_scenario_identity_is_blocked() {
        let mut input = valid_water_input();
        input.scenario_id.clear();
        let decision = evaluate_resource_balance(&input);
        assert!(
            decision
                .blockers
                .iter()
                .any(|item| { item.code == BalanceBlockerCode::EmptyScenarioId })
        );
    }

    #[test]
    fn wide_integer_arithmetic_preserves_large_balances_exactly() {
        let mut input = valid_water_input();
        input.inflow = u64::MAX;
        input.generated_or_added = u64::MAX;
        input.outflow = 0;
        input.declared_loss = 0;
        input.observed_storage_delta = 0;
        input.tolerance = 0;

        let decision = evaluate_resource_balance(&input);
        assert_eq!(decision.expected_storage_delta, i128::from(u64::MAX) * 2);
        assert_eq!(decision.absolute_error, u128::from(u64::MAX) * 2);
        assert_eq!(decision.disposition, BalanceDisposition::Blocked);
    }

    #[test]
    fn exact_zero_balance_can_use_zero_tolerance() {
        let mut input = valid_water_input();
        input.inflow = 1_000;
        input.outflow = 1_000;
        input.declared_loss = 0;
        input.observed_storage_delta = 0;
        input.tolerance = 0;
        let decision = evaluate_resource_balance(&input);
        assert_eq!(
            decision.disposition,
            BalanceDisposition::AccountingConsistentForDeclaredBoundary
        );
        assert_eq!(decision.absolute_error, 0);
    }

    #[test]
    fn input_and_decision_have_stable_json_roundtrip() {
        let input = valid_water_input();
        let serialized = serde_json::to_string(&input).expect("input serializes");
        let decoded: ResourceBalanceInputV1 =
            serde_json::from_str(&serialized).expect("input deserializes");
        assert_eq!(decoded, input);

        let decision = evaluate_resource_balance(&input);
        let serialized = serde_json::to_string(&decision).expect("decision serializes");
        let decoded: ResourceBalanceDecisionV1 =
            serde_json::from_str(&serialized).expect("decision deserializes");
        assert_eq!(decoded, decision);
    }

    #[test]
    fn wide_integer_wire_values_are_decimal_strings_and_roundtrip_exactly() {
        let mut input = valid_water_input();
        input.inflow = u64::MAX;
        input.generated_or_added = u64::MAX;
        input.outflow = 0;
        input.declared_loss = 0;
        input.observed_storage_delta = i128::MIN;
        input.tolerance = 0;

        let serialized = serde_json::to_string(&input).expect("input serializes");
        assert!(serialized.contains("\"inflow\":\"18446744073709551615\""));
        assert!(serialized.contains(
            "\"observed_storage_delta\":\"-170141183460469231731687303715884105728\""
        ));
        let decoded: ResourceBalanceInputV1 =
            serde_json::from_str(&serialized).expect("input deserializes");
        assert_eq!(decoded, input);

        let decision = evaluate_resource_balance(&input);
        let serialized_decision =
            serde_json::to_string(&decision).expect("decision serializes");
        assert!(serialized_decision.contains(&format!(
            "\"expected_storage_delta\":\"{}\"",
            decision.expected_storage_delta
        )));
        assert!(serialized_decision.contains(&format!(
            "\"absolute_error\":\"{}\"",
            decision.absolute_error
        )));
        let decoded_decision: ResourceBalanceDecisionV1 =
            serde_json::from_str(&serialized_decision).expect("decision deserializes");
        assert_eq!(decoded_decision, decision);
    }

    fn valid_nutrient_fixture() -> (NutrientCompositionRecordV1, EdibleFoodDeliveryV1) {
        let composition = NutrientCompositionRecordV1 {
            schema_version: GARDEN_RESOURCE_SCHEMA_VERSION,
            food_record_id: "FOOD-001".to_owned(),
            food_state_id: "boiled_edible".to_owned(),
            nutrient_id: "iron".to_owned(),
            nutrient_definition_ref: "nutrient-definition:iron-v1".to_owned(),
            unit: NutrientUnit::Milligrams,
            amount_per_100g_scaled: Some(250_000),
            value_scale: 1_000,
            evidence_class: NutrientCompositionEvidenceClass::SyntheticFixture,
            source_ref: "fixture:composition-001".to_owned(),
        };
        let delivery = EdibleFoodDeliveryV1 {
            schema_version: GARDEN_RESOURCE_SCHEMA_VERSION,
            scenario_id: "SYNTH-FOOD-001".to_owned(),
            food_record_id: "FOOD-001".to_owned(),
            food_state_id: "boiled_edible".to_owned(),
            edible_delivered_grams: 75,
            delivery_source_ref: "fixture:delivered-edible-mass-001".to_owned(),
        };
        (composition, delivery)
    }

    #[test]
    fn nutrient_contribution_preserves_exact_rational_value() {
        let (composition, delivery) = valid_nutrient_fixture();
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert_eq!(
            decision.disposition,
            NutrientContributionDisposition::CalculatedForDeclaredDelivery
        );
        assert_eq!(decision.contribution_numerator, Some(18_750_000));
        assert_eq!(decision.contribution_denominator, Some(100_000));
        assert_eq!(decision.unit, NutrientUnit::Milligrams);
        assert_eq!(decision.composition_source_ref, "fixture:composition-001");
        assert_eq!(decision.delivery_source_ref, "fixture:delivered-edible-mass-001");
        // 18,750,000 / 100,000 = 187.5 mg; no float or rounding is used.
    }

    #[test]
    fn different_food_record_is_blocked() {
        let (composition, mut delivery) = valid_nutrient_fixture();
        delivery.food_record_id = "FOOD-OTHER".to_owned();
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert_eq!(decision.disposition, NutrientContributionDisposition::Blocked);
        assert!(decision.blockers.iter().any(|item| {
            item.code == NutrientContributionBlockerCode::FoodRecordMismatch
        }));
        assert_eq!(decision.contribution_numerator, None);
    }

    #[test]
    fn different_preparation_state_is_blocked() {
        let (composition, mut delivery) = valid_nutrient_fixture();
        delivery.food_state_id = "raw_edible".to_owned();
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert!(decision.blockers.iter().any(|item| {
            item.code == NutrientContributionBlockerCode::FoodStateMismatch
        }));
        assert_eq!(decision.disposition, NutrientContributionDisposition::Blocked);
    }

    #[test]
    fn missing_composition_is_not_assumed_to_be_zero() {
        let (mut composition, delivery) = valid_nutrient_fixture();
        composition.amount_per_100g_scaled = None;
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert_eq!(decision.disposition, NutrientContributionDisposition::NotAssessable);
        assert_eq!(decision.contribution_numerator, None);
        assert_eq!(decision.contribution_denominator, None);
        assert!(decision.blockers.iter().any(|item| {
            item.code == NutrientContributionBlockerCode::MissingCompositionValue
        }));
    }

    #[test]
    fn zero_value_scale_is_blocked() {
        let (mut composition, delivery) = valid_nutrient_fixture();
        composition.value_scale = 0;
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert_eq!(decision.disposition, NutrientContributionDisposition::Blocked);
        assert_eq!(decision.contribution_numerator, None);
        assert!(decision.blockers.iter().any(|item| {
            item.code == NutrientContributionBlockerCode::ZeroValueScale
        }));
    }

    #[test]
    fn extreme_nutrient_amount_and_mass_remain_exact() {
        let (mut composition, mut delivery) = valid_nutrient_fixture();
        composition.amount_per_100g_scaled = Some(u64::MAX);
        composition.value_scale = u64::MAX;
        delivery.edible_delivered_grams = u64::MAX;
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert_eq!(
            decision.contribution_numerator,
            Some(u128::from(u64::MAX) * u128::from(u64::MAX))
        );
        assert_eq!(
            decision.contribution_denominator,
            Some(100 * u128::from(u64::MAX))
        );
        assert_eq!(
            decision.disposition,
            NutrientContributionDisposition::CalculatedForDeclaredDelivery
        );
    }

    #[test]
    fn nutrient_contribution_json_preserves_large_integers_as_strings() {
        let (mut composition, mut delivery) = valid_nutrient_fixture();
        composition.amount_per_100g_scaled = Some(u64::MAX);
        composition.value_scale = u64::MAX;
        delivery.edible_delivered_grams = u64::MAX;
        let serialized_input = serde_json::to_string(&composition).expect("input serializes");
        assert!(serialized_input.contains("\"amount_per_100g_scaled\":\"18446744073709551615\""));
        let decoded_input: NutrientCompositionRecordV1 =
            serde_json::from_str(&serialized_input).expect("input deserializes");
        assert_eq!(decoded_input, composition);

        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        let serialized_decision = serde_json::to_string(&decision).expect("decision serializes");
        assert!(serialized_decision.contains(&format!(
            "\"contribution_numerator\":\"{}\"",
            u128::from(u64::MAX) * u128::from(u64::MAX)
        )));
        let decoded_decision: NutrientContributionDecisionV1 =
            serde_json::from_str(&serialized_decision).expect("decision deserializes");
        assert_eq!(decoded_decision, decision);
    }

    #[test]
    fn missing_provenance_blocks_nutrient_calculation() {
        let (mut composition, mut delivery) = valid_nutrient_fixture();
        composition.source_ref.clear();
        delivery.delivery_source_ref.clear();
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert_eq!(decision.disposition, NutrientContributionDisposition::Blocked);
        assert_eq!(decision.contribution_numerator, None);
        assert!(decision.blockers.iter().any(|item| {
            item.code == NutrientContributionBlockerCode::MissingCompositionSource
        }));
        assert!(decision.blockers.iter().any(|item| {
            item.code == NutrientContributionBlockerCode::MissingDeliverySource
        }));
    }

}

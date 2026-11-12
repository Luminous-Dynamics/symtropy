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
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub inflow: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub generated_or_added: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub outflow: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub declared_loss: u64,
    #[serde(
        serialize_with = "serialize_i128_as_string",
        deserialize_with = "deserialize_i128_from_string"
    )]
    pub observed_storage_delta: i128,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
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
    #[serde(
        serialize_with = "serialize_i128_as_string",
        deserialize_with = "deserialize_i128_from_string"
    )]
    pub expected_storage_delta: i128,
    #[serde(
        serialize_with = "serialize_u128_as_string",
        deserialize_with = "deserialize_u128_from_string"
    )]
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

fn push_blocker(blockers: &mut Vec<BalanceBlocker>, code: BalanceBlockerCode, detail: &str) {
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
    /// Exact beneficiary/community scope for this delivered amount.
    pub delivery_scope_ref: String,
    /// Period covered by the amount; must match the target comparison period.
    pub period_ref: String,
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
    EmptyDeliveryScope,
    EmptyDeliveryPeriod,
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
    pub delivery_scope_ref: String,
    pub period_ref: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub edible_delivered_grams: u64,
    pub nutrient_id: String,
    pub nutrient_definition_ref: String,
    pub unit: NutrientUnit,
    #[serde(
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string"
    )]
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
    if delivery.delivery_scope_ref.trim().is_empty() {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::EmptyDeliveryScope,
            "delivery scope reference is empty",
        );
    }
    if delivery.period_ref.trim().is_empty() {
        push_nutrient_blocker(
            &mut blockers,
            NutrientContributionBlockerCode::EmptyDeliveryPeriod,
            "delivery period reference is empty",
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

    let contribution_numerator =
        if disposition == NutrientContributionDisposition::CalculatedForDeclaredDelivery {
            composition
                .amount_per_100g_scaled
                .map(|amount| u128::from(amount) * u128::from(delivery.edible_delivered_grams))
        } else {
            None
        };
    let contribution_denominator =
        if disposition == NutrientContributionDisposition::CalculatedForDeclaredDelivery {
            Some(100 * u128::from(composition.value_scale))
        } else {
            None
        };

    NutrientContributionDecisionV1 {
        scenario_id: delivery.scenario_id.clone(),
        food_record_id: composition.food_record_id.clone(),
        food_state_id: composition.food_state_id.clone(),
        delivery_scope_ref: delivery.delivery_scope_ref.clone(),
        period_ref: delivery.period_ref.clone(),
        edible_delivered_grams: delivery.edible_delivered_grams,
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

fn deserialize_option_u64_from_string<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
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

fn deserialize_option_u128_from_string<'de, D>(deserializer: D) -> Result<Option<u128>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|raw| raw.parse::<u128>().map_err(serde::de::Error::custom))
        .transpose()
}

/// A minimum nutrient-supply target for a declared population/scope and period.
/// This is a planning input, not an individual dietary reference or clinical target.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientSupplyTargetV1 {
    pub schema_version: u16,
    pub target_id: String,
    pub nutrient_id: String,
    pub nutrient_definition_ref: String,
    pub unit: NutrientUnit,
    /// Minimum nutrient amount over target_scope_ref and period_ref, expressed
    /// as target_amount_scaled / value_scale in the declared NutrientUnit.
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub target_amount_scaled: u64,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub value_scale: u64,
    /// Opaque reference to an accountable community/population scope.
    pub target_scope_ref: String,
    /// Opaque reference to the period the target covers.
    pub period_ref: String,
    /// Source for the declared target; not authenticated here.
    pub source_ref: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientTargetBlockerCode {
    UnsupportedTargetSchema,
    EmptyTargetId,
    EmptyNutrientId,
    EmptyNutrientDefinitionReference,
    EmptyTargetScope,
    EmptyTargetPeriod,
    EmptyTargetSource,
    ZeroTargetValue,
    ZeroTargetScale,
    NutrientMismatch,
    NutrientDefinitionMismatch,
    NutrientUnitMismatch,
    NutrientScaleMismatch,
    TargetScopeMismatch,
    TargetPeriodMismatch,
    ContributionBlocked,
}

/// Stable blocker for a source-scoped nutrient-target comparison.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientTargetBlocker {
    pub code: NutrientTargetBlockerCode,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientTargetStatus {
    MeetsDeclaredMinimum,
    ShortfallAgainstDeclaredMinimum,
    NotAssessable,
    Blocked,
}

/// Exact, source-scoped comparison of one food delivery with one minimum target.
/// supply_numerator / denominator and target_numerator / denominator share a
/// common denominator, so the comparison itself needs neither floats nor rounding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientTargetAssessmentV1 {
    pub target_id: String,
    pub scenario_id: String,
    pub nutrient_id: String,
    pub nutrient_definition_ref: String,
    pub unit: NutrientUnit,
    pub delivery_scope_ref: String,
    pub delivery_period_ref: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub edible_delivered_grams: u64,
    pub target_scope_ref: String,
    pub period_ref: String,
    #[serde(
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string"
    )]
    pub value_scale: Option<u64>,
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub supply_numerator: Option<u128>,
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub target_numerator: Option<u128>,
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub denominator: Option<u128>,
    /// The shortfall in the same numerator scale; zero when the declared target is met.
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub shortfall_numerator: Option<u128>,
    pub composition_source_ref: String,
    pub delivery_source_ref: String,
    pub target_source_ref: String,
    pub status: NutrientTargetStatus,
    pub blockers: Vec<NutrientTargetBlocker>,
}

/// Compare a source-scoped nutrient contribution to an externally defined minimum.
///
/// All target and composition values must use the same nutrient definition, base
/// unit and decimal scale. Population/life-stage reference requirements and target
/// derivation are outside this calculator. Missing composition stays not assessable.
pub fn assess_nutrient_target(
    composition: &NutrientCompositionRecordV1,
    delivery: &EdibleFoodDeliveryV1,
    target: &NutrientSupplyTargetV1,
) -> NutrientTargetAssessmentV1 {
    let contribution = evaluate_nutrient_contribution(composition, delivery);
    let mut blockers = Vec::new();

    if target.schema_version != GARDEN_RESOURCE_SCHEMA_VERSION {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::UnsupportedTargetSchema,
            "unsupported nutrient-target schema version",
        );
    }
    if target.target_id.trim().is_empty() {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::EmptyTargetId,
            "target identity is empty",
        );
    }
    if target.nutrient_id.trim().is_empty() {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::EmptyNutrientId,
            "target nutrient identity is empty",
        );
    }
    if target.nutrient_definition_ref.trim().is_empty() {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::EmptyNutrientDefinitionReference,
            "target nutrient-definition reference is empty",
        );
    }
    if target.target_scope_ref.trim().is_empty() {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::EmptyTargetScope,
            "population/community scope is required",
        );
    }
    if target.period_ref.trim().is_empty() {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::EmptyTargetPeriod,
            "target time-period reference is required",
        );
    }
    if target.source_ref.trim().is_empty() {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::EmptyTargetSource,
            "target source reference is required",
        );
    }
    if target.target_amount_scaled == 0 {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::ZeroTargetValue,
            "a positive minimum target is required by this contract",
        );
    }
    if target.value_scale == 0 {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::ZeroTargetScale,
            "target value scale must be greater than zero",
        );
    }
    if target.nutrient_id != composition.nutrient_id {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::NutrientMismatch,
            "target and composition refer to different nutrients",
        );
    }
    if target.nutrient_definition_ref != composition.nutrient_definition_ref {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::NutrientDefinitionMismatch,
            "target and composition use different nutrient definitions",
        );
    }
    if target.unit != composition.unit {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::NutrientUnitMismatch,
            "target and composition units differ",
        );
    }
    if target.value_scale != composition.value_scale {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::NutrientScaleMismatch,
            "target and composition decimal scales differ",
        );
    }
    if target.target_scope_ref != delivery.delivery_scope_ref {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::TargetScopeMismatch,
            "nutrient target and delivered food use different population/community scopes",
        );
    }
    if target.period_ref != delivery.period_ref {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::TargetPeriodMismatch,
            "nutrient target and delivered food cover different periods",
        );
    }
    if contribution.disposition == NutrientContributionDisposition::Blocked {
        push_target_blocker(
            &mut blockers,
            NutrientTargetBlockerCode::ContributionBlocked,
            "source-scoped nutrient contribution is blocked",
        );
    }

    let hard_blocked = !blockers.is_empty();
    let assessable =
        contribution.disposition == NutrientContributionDisposition::CalculatedForDeclaredDelivery;
    let denominator = if !hard_blocked && assessable {
        Some(100 * u128::from(target.value_scale))
    } else {
        None
    };
    let supply_numerator = if denominator.is_some() {
        contribution.contribution_numerator
    } else {
        None
    };
    let target_numerator = if denominator.is_some() {
        Some(100 * u128::from(target.target_amount_scaled))
    } else {
        None
    };
    let shortfall_numerator = match (supply_numerator, target_numerator) {
        (Some(supply), Some(minimum)) => Some(minimum.saturating_sub(supply)),
        _ => None,
    };
    let status = if hard_blocked {
        NutrientTargetStatus::Blocked
    } else if !assessable {
        NutrientTargetStatus::NotAssessable
    } else {
        match (supply_numerator, target_numerator) {
            (Some(supply), Some(minimum)) if supply >= minimum => {
                NutrientTargetStatus::MeetsDeclaredMinimum
            }
            (Some(_), Some(_)) => NutrientTargetStatus::ShortfallAgainstDeclaredMinimum,
            _ => NutrientTargetStatus::Blocked,
        }
    };

    NutrientTargetAssessmentV1 {
        target_id: target.target_id.clone(),
        scenario_id: delivery.scenario_id.clone(),
        nutrient_id: target.nutrient_id.clone(),
        nutrient_definition_ref: target.nutrient_definition_ref.clone(),
        unit: target.unit,
        delivery_scope_ref: delivery.delivery_scope_ref.clone(),
        delivery_period_ref: delivery.period_ref.clone(),
        edible_delivered_grams: delivery.edible_delivered_grams,
        target_scope_ref: target.target_scope_ref.clone(),
        period_ref: target.period_ref.clone(),
        value_scale: (target.value_scale != 0).then_some(target.value_scale),
        supply_numerator,
        target_numerator,
        denominator,
        shortfall_numerator,
        composition_source_ref: composition.source_ref.clone(),
        delivery_source_ref: delivery.delivery_source_ref.clone(),
        target_source_ref: target.source_ref.clone(),
        status,
        blockers,
    }
}

fn push_target_blocker(
    blockers: &mut Vec<NutrientTargetBlocker>,
    code: NutrientTargetBlockerCode,
    detail: &str,
) {
    blockers.push(NutrientTargetBlocker {
        code,
        detail: detail.to_owned(),
    });
}

/// One source-pinned food-composition record paired with one delivered edible mass.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientSupplyLineV1 {
    pub composition: NutrientCompositionRecordV1,
    pub delivery: EdibleFoodDeliveryV1,
}

/// A candidate supply scenario for one population/community and one period.
/// It describes inputs to the calculator, not a committed distribution or diet.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientSupplyScenarioV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub target_scope_ref: String,
    pub period_ref: String,
    pub lines: Vec<NutrientSupplyLineV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientPortfolioBlockerCode {
    UnsupportedScenarioSchema,
    EmptyScenarioId,
    EmptyTargetScope,
    EmptyTargetPeriod,
    NoSupplyLines,
    ScenarioMismatch,
    ScopeMismatch,
    PeriodMismatch,
    DuplicateDeliverySource,
    InvalidContributionLine,
    MissingCompositionValue,
    QuantityOverflow,
}

/// A stable blocker. line_index is zero-based and omitted for scenario-level blockers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientPortfolioBlockerV1 {
    pub line_index: Option<usize>,
    pub code: NutrientPortfolioBlockerCode,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NutrientPortfolioStatus {
    MeetsDeclaredMinimum,
    ShortfallAgainstDeclaredMinimum,
    NotAssessable,
    Blocked,
}

/// Aggregate exact nutrient supply from multiple food lines against one target.
///
/// Every line must share a nutrient definition, unit, scale, scenario, population
/// scope and period. Missing composition prevents a partial sum from being
/// presented as complete. Duplicate delivery source refs and arithmetic overflow
/// block the scenario. Composition source refs may repeat because multiple rows can
/// legitimately come from one licensed table artifact.
/// Per-delivery receipt keeps exact food identity, delivered mass, source refs
/// and the individual nutrient contribution available for independent audit.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientSupplyLineReceiptV1 {
    pub line_index: usize,
    pub food_record_id: String,
    pub food_state_id: String,
    #[serde(
        serialize_with = "serialize_u64_as_string",
        deserialize_with = "deserialize_u64_from_string"
    )]
    pub edible_delivered_grams: u64,
    pub composition_source_ref: String,
    pub delivery_source_ref: String,
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
    pub target_status: NutrientTargetStatus,
    pub target_blockers: Vec<NutrientTargetBlocker>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutrientPortfolioAssessmentV1 {
    pub scenario_id: String,
    pub target_id: String,
    pub nutrient_id: String,
    pub nutrient_definition_ref: String,
    pub unit: NutrientUnit,
    pub target_scope_ref: String,
    pub period_ref: String,
    #[serde(
        serialize_with = "serialize_option_u64_as_string",
        deserialize_with = "deserialize_option_u64_from_string"
    )]
    pub value_scale: Option<u64>,
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub total_edible_delivered_grams: Option<u128>,
    /// Sum of line numerators with common denominator 100 * value_scale.
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub supply_numerator: Option<u128>,
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub target_numerator: Option<u128>,
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub denominator: Option<u128>,
    #[serde(
        serialize_with = "serialize_option_u128_as_string",
        deserialize_with = "deserialize_option_u128_from_string"
    )]
    pub shortfall_numerator: Option<u128>,
    pub composition_source_refs: Vec<String>,
    pub delivery_source_refs: Vec<String>,
    pub line_receipts: Vec<NutrientSupplyLineReceiptV1>,
    pub target_source_ref: String,
    pub status: NutrientPortfolioStatus,
    pub blockers: Vec<NutrientPortfolioBlockerV1>,
}

/// Independently aggregate multiple source-attributed food lines against one declared
/// nutrient target. This calculator does not choose crops, estimate yield, infer
/// composition, authenticate references, or derive clinical dietary requirements.
pub fn evaluate_nutrient_supply_scenario(
    scenario: &NutrientSupplyScenarioV1,
    target: &NutrientSupplyTargetV1,
) -> NutrientPortfolioAssessmentV1 {
    let mut blockers = Vec::new();
    let mut seen_delivery_refs: Vec<String> = Vec::new();
    let mut composition_refs: Vec<String> = Vec::new();
    let mut delivery_refs: Vec<String> = Vec::new();
    let mut line_receipts: Vec<NutrientSupplyLineReceiptV1> = Vec::new();
    let mut supply_numerator = 0_u128;
    let mut total_edible_grams = 0_u128;
    let mut has_missing_composition = false;
    let mut has_quantity_overflow = false;

    if scenario.schema_version != GARDEN_RESOURCE_SCHEMA_VERSION {
        push_portfolio_blocker(
            &mut blockers,
            None,
            NutrientPortfolioBlockerCode::UnsupportedScenarioSchema,
            "unsupported nutrient supply-scenario schema version",
        );
    }
    if scenario.scenario_id.trim().is_empty() {
        push_portfolio_blocker(
            &mut blockers,
            None,
            NutrientPortfolioBlockerCode::EmptyScenarioId,
            "supply scenario identity is empty",
        );
    }
    if scenario.target_scope_ref.trim().is_empty() {
        push_portfolio_blocker(
            &mut blockers,
            None,
            NutrientPortfolioBlockerCode::EmptyTargetScope,
            "supply scenario scope reference is empty",
        );
    }
    if scenario.period_ref.trim().is_empty() {
        push_portfolio_blocker(
            &mut blockers,
            None,
            NutrientPortfolioBlockerCode::EmptyTargetPeriod,
            "supply scenario period reference is empty",
        );
    }
    if scenario.lines.is_empty() {
        push_portfolio_blocker(
            &mut blockers,
            None,
            NutrientPortfolioBlockerCode::NoSupplyLines,
            "no food delivery lines were supplied; absence is not assumed to mean zero production",
        );
    }

    if target.target_scope_ref != scenario.target_scope_ref {
        push_portfolio_blocker(
            &mut blockers,
            None,
            NutrientPortfolioBlockerCode::ScopeMismatch,
            "target and supply scenario name different population/community scopes",
        );
    }
    if target.period_ref != scenario.period_ref {
        push_portfolio_blocker(
            &mut blockers,
            None,
            NutrientPortfolioBlockerCode::PeriodMismatch,
            "target and supply scenario cover different periods",
        );
    }

    for (line_index, line) in scenario.lines.iter().enumerate() {
        let single = assess_nutrient_target(&line.composition, &line.delivery, target);
        line_receipts.push(NutrientSupplyLineReceiptV1 {
            line_index,
            food_record_id: line.composition.food_record_id.clone(),
            food_state_id: line.composition.food_state_id.clone(),
            edible_delivered_grams: line.delivery.edible_delivered_grams,
            composition_source_ref: line.composition.source_ref.clone(),
            delivery_source_ref: line.delivery.delivery_source_ref.clone(),
            contribution_numerator: single.supply_numerator,
            contribution_denominator: single.denominator,
            target_status: single.status,
            target_blockers: single.blockers.clone(),
        });
        if line.delivery.scenario_id != scenario.scenario_id {
            push_portfolio_blocker(
                &mut blockers,
                Some(line_index),
                NutrientPortfolioBlockerCode::ScenarioMismatch,
                "delivery row belongs to a different scenario",
            );
        }
        if line.delivery.delivery_scope_ref != scenario.target_scope_ref {
            push_portfolio_blocker(
                &mut blockers,
                Some(line_index),
                NutrientPortfolioBlockerCode::ScopeMismatch,
                "delivery row uses a different population/community scope",
            );
        }
        if line.delivery.period_ref != scenario.period_ref {
            push_portfolio_blocker(
                &mut blockers,
                Some(line_index),
                NutrientPortfolioBlockerCode::PeriodMismatch,
                "delivery row covers a different time period",
            );
        }

        let delivery_source_ref = line.delivery.delivery_source_ref.trim();
        if !delivery_source_ref.is_empty() {
            if seen_delivery_refs
                .iter()
                .any(|existing| existing.as_str() == delivery_source_ref)
            {
                push_portfolio_blocker(
                    &mut blockers,
                    Some(line_index),
                    NutrientPortfolioBlockerCode::DuplicateDeliverySource,
                    "delivery source reference is repeated; counting it again could double-count supply",
                );
            } else {
                seen_delivery_refs.push(delivery_source_ref.to_owned());
            }
        }

        add_unique_source(&mut composition_refs, &line.composition.source_ref);
        add_unique_source(&mut delivery_refs, &line.delivery.delivery_source_ref);

        if single.status == NutrientTargetStatus::Blocked {
            let detail = single
                .blockers
                .iter()
                .map(|blocker| blocker.detail.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            push_portfolio_blocker(
                &mut blockers,
                Some(line_index),
                NutrientPortfolioBlockerCode::InvalidContributionLine,
                &detail,
            );
            continue;
        }
        if single.status == NutrientTargetStatus::NotAssessable {
            has_missing_composition = true;
            push_portfolio_blocker(
                &mut blockers,
                Some(line_index),
                NutrientPortfolioBlockerCode::MissingCompositionValue,
                "one delivery line has no assessable composition; partial totals are withheld",
            );
            continue;
        }

        let Some(line_numerator) = single.supply_numerator else {
            push_portfolio_blocker(
                &mut blockers,
                Some(line_index),
                NutrientPortfolioBlockerCode::InvalidContributionLine,
                "assessable nutrient line did not produce an exact numerator",
            );
            continue;
        };

        match supply_numerator.checked_add(line_numerator) {
            Some(total) => supply_numerator = total,
            None => {
                has_quantity_overflow = true;
                push_portfolio_blocker(
                    &mut blockers,
                    Some(line_index),
                    NutrientPortfolioBlockerCode::QuantityOverflow,
                    "aggregated nutrient numerator exceeds u128",
                );
            }
        }
        match total_edible_grams.checked_add(u128::from(line.delivery.edible_delivered_grams)) {
            Some(total) => total_edible_grams = total,
            None => {
                has_quantity_overflow = true;
                push_portfolio_blocker(
                    &mut blockers,
                    Some(line_index),
                    NutrientPortfolioBlockerCode::QuantityOverflow,
                    "aggregated edible delivered mass exceeds u128",
                );
            }
        }
    }

    let hard_blocked = blockers.iter().any(|blocker| {
        !matches!(
            blocker.code,
            NutrientPortfolioBlockerCode::NoSupplyLines
                | NutrientPortfolioBlockerCode::MissingCompositionValue
        )
    });
    if has_quantity_overflow && !hard_blocked {
        // Defensive consistency: overflow must always be a hard blocker.
        push_portfolio_blocker(
            &mut blockers,
            None,
            NutrientPortfolioBlockerCode::QuantityOverflow,
            "aggregate arithmetic did not remain representable",
        );
    }
    let has_lines = !scenario.lines.is_empty();
    let fully_assessable =
        has_lines && !has_missing_composition && !has_quantity_overflow && !hard_blocked;
    let denominator = if fully_assessable {
        Some(100 * u128::from(target.value_scale))
    } else {
        None
    };
    let aggregate_numerator = if fully_assessable {
        Some(supply_numerator)
    } else {
        None
    };
    let target_numerator = if fully_assessable {
        Some(100 * u128::from(target.target_amount_scaled))
    } else {
        None
    };
    let shortfall_numerator = match (aggregate_numerator, target_numerator) {
        (Some(supply), Some(minimum)) => Some(minimum.saturating_sub(supply)),
        _ => None,
    };

    let status = if hard_blocked {
        NutrientPortfolioStatus::Blocked
    } else if !fully_assessable {
        NutrientPortfolioStatus::NotAssessable
    } else {
        match (aggregate_numerator, target_numerator) {
            (Some(supply), Some(minimum)) if supply >= minimum => {
                NutrientPortfolioStatus::MeetsDeclaredMinimum
            }
            (Some(_), Some(_)) => NutrientPortfolioStatus::ShortfallAgainstDeclaredMinimum,
            _ => NutrientPortfolioStatus::Blocked,
        }
    };

    NutrientPortfolioAssessmentV1 {
        scenario_id: scenario.scenario_id.clone(),
        target_id: target.target_id.clone(),
        nutrient_id: target.nutrient_id.clone(),
        nutrient_definition_ref: target.nutrient_definition_ref.clone(),
        unit: target.unit,
        target_scope_ref: scenario.target_scope_ref.clone(),
        period_ref: scenario.period_ref.clone(),
        value_scale: (target.value_scale != 0).then_some(target.value_scale),
        total_edible_delivered_grams: fully_assessable.then_some(total_edible_grams),
        supply_numerator: aggregate_numerator,
        target_numerator,
        denominator,
        shortfall_numerator,
        composition_source_refs: composition_refs,
        delivery_source_refs: delivery_refs,
        line_receipts,
        target_source_ref: target.source_ref.clone(),
        status,
        blockers,
    }
}

fn add_unique_source(sources: &mut Vec<String>, source: &str) {
    if !source.trim().is_empty() && !sources.iter().any(|existing| existing == source) {
        sources.push(source.to_owned());
    }
}

fn push_portfolio_blocker(
    blockers: &mut Vec<NutrientPortfolioBlockerV1>,
    line_index: Option<usize>,
    code: NutrientPortfolioBlockerCode,
    detail: &str,
) {
    blockers.push(NutrientPortfolioBlockerV1 {
        line_index,
        code,
        detail: detail.to_owned(),
    });
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
        assert!(
            serialized.contains(
                "\"observed_storage_delta\":\"-170141183460469231731687303715884105728\""
            )
        );
        let decoded: ResourceBalanceInputV1 =
            serde_json::from_str(&serialized).expect("input deserializes");
        assert_eq!(decoded, input);

        let decision = evaluate_resource_balance(&input);
        let serialized_decision = serde_json::to_string(&decision).expect("decision serializes");
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
            delivery_scope_ref: "scope:community-001".to_owned(),
            period_ref: "period:2026-11".to_owned(),
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
        assert_eq!(decision.edible_delivered_grams, 75);
        assert_eq!(decision.composition_source_ref, "fixture:composition-001");
        assert_eq!(
            decision.delivery_source_ref,
            "fixture:delivered-edible-mass-001"
        );
        // 18,750,000 / 100,000 = 187.5 mg; no float or rounding is used.
    }

    #[test]
    fn different_food_record_is_blocked() {
        let (composition, mut delivery) = valid_nutrient_fixture();
        delivery.food_record_id = "FOOD-OTHER".to_owned();
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert_eq!(
            decision.disposition,
            NutrientContributionDisposition::Blocked
        );
        assert!(
            decision
                .blockers
                .iter()
                .any(|item| { item.code == NutrientContributionBlockerCode::FoodRecordMismatch })
        );
        assert_eq!(decision.contribution_numerator, None);
    }

    #[test]
    fn different_preparation_state_is_blocked() {
        let (composition, mut delivery) = valid_nutrient_fixture();
        delivery.food_state_id = "raw_edible".to_owned();
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert!(
            decision
                .blockers
                .iter()
                .any(|item| { item.code == NutrientContributionBlockerCode::FoodStateMismatch })
        );
        assert_eq!(
            decision.disposition,
            NutrientContributionDisposition::Blocked
        );
    }

    #[test]
    fn missing_composition_is_not_assumed_to_be_zero() {
        let (mut composition, delivery) = valid_nutrient_fixture();
        composition.amount_per_100g_scaled = None;
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert_eq!(
            decision.disposition,
            NutrientContributionDisposition::NotAssessable
        );
        assert_eq!(decision.contribution_numerator, None);
        assert_eq!(decision.contribution_denominator, None);
        assert!(
            decision.blockers.iter().any(|item| {
                item.code == NutrientContributionBlockerCode::MissingCompositionValue
            })
        );
    }

    #[test]
    fn zero_value_scale_is_blocked() {
        let (mut composition, delivery) = valid_nutrient_fixture();
        composition.value_scale = 0;
        let decision = evaluate_nutrient_contribution(&composition, &delivery);
        assert_eq!(
            decision.disposition,
            NutrientContributionDisposition::Blocked
        );
        assert_eq!(decision.contribution_numerator, None);
        assert!(
            decision
                .blockers
                .iter()
                .any(|item| { item.code == NutrientContributionBlockerCode::ZeroValueScale })
        );
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
        assert_eq!(
            decision.disposition,
            NutrientContributionDisposition::Blocked
        );
        assert_eq!(decision.contribution_numerator, None);
        assert!(decision.blockers.iter().any(|item| {
            item.code == NutrientContributionBlockerCode::MissingCompositionSource
        }));
        assert!(
            decision.blockers.iter().any(|item| {
                item.code == NutrientContributionBlockerCode::MissingDeliverySource
            })
        );
    }

    fn valid_nutrient_target() -> NutrientSupplyTargetV1 {
        NutrientSupplyTargetV1 {
            schema_version: GARDEN_RESOURCE_SCHEMA_VERSION,
            target_id: "TARGET-COMMUNITY-IRON-001".to_owned(),
            nutrient_id: "iron".to_owned(),
            nutrient_definition_ref: "nutrient-definition:iron-v1".to_owned(),
            unit: NutrientUnit::Milligrams,
            target_amount_scaled: 180_000,
            value_scale: 1_000,
            target_scope_ref: "scope:community-001".to_owned(),
            period_ref: "period:2026-11".to_owned(),
            source_ref: "fixture:reviewed-community-target-001".to_owned(),
        }
    }

    #[test]
    fn nutrient_target_meets_exact_minimum_without_rounding() {
        let (composition, delivery) = valid_nutrient_fixture();
        let target = valid_nutrient_target();
        let assessment = assess_nutrient_target(&composition, &delivery, &target);
        assert_eq!(
            assessment.status,
            NutrientTargetStatus::MeetsDeclaredMinimum
        );
        assert_eq!(assessment.supply_numerator, Some(18_750_000));
        assert_eq!(assessment.target_numerator, Some(18_000_000));
        assert_eq!(assessment.denominator, Some(100_000));
        assert_eq!(assessment.shortfall_numerator, Some(0));
        assert_eq!(assessment.delivery_scope_ref, "scope:community-001");
        assert_eq!(assessment.delivery_period_ref, "period:2026-11");
        assert_eq!(assessment.edible_delivered_grams, 75);
        assert_eq!(assessment.target_scope_ref, "scope:community-001");
        assert_eq!(assessment.period_ref, "period:2026-11");
    }

    #[test]
    fn nutrient_target_reports_exact_shortfall() {
        let (composition, delivery) = valid_nutrient_fixture();
        let mut target = valid_nutrient_target();
        target.target_amount_scaled = 200_000;
        let assessment = assess_nutrient_target(&composition, &delivery, &target);
        assert_eq!(
            assessment.status,
            NutrientTargetStatus::ShortfallAgainstDeclaredMinimum
        );
        assert_eq!(assessment.supply_numerator, Some(18_750_000));
        assert_eq!(assessment.target_numerator, Some(20_000_000));
        assert_eq!(assessment.shortfall_numerator, Some(1_250_000));
        // 1,250,000 / 100,000 = 12.5 mg shortfall.
    }

    #[test]
    fn nutrient_target_with_unknown_composition_is_not_assessable() {
        let (mut composition, delivery) = valid_nutrient_fixture();
        composition.amount_per_100g_scaled = None;
        let assessment = assess_nutrient_target(&composition, &delivery, &valid_nutrient_target());
        assert_eq!(assessment.status, NutrientTargetStatus::NotAssessable);
        assert_eq!(assessment.supply_numerator, None);
        assert_eq!(assessment.target_numerator, None);
    }

    #[test]
    fn target_unit_mismatch_is_blocked() {
        let (composition, delivery) = valid_nutrient_fixture();
        let mut target = valid_nutrient_target();
        target.unit = NutrientUnit::Grams;
        let assessment = assess_nutrient_target(&composition, &delivery, &target);
        assert_eq!(assessment.status, NutrientTargetStatus::Blocked);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientTargetBlockerCode::NutrientUnitMismatch })
        );
        assert_eq!(assessment.supply_numerator, None);
    }

    #[test]
    fn target_scale_mismatch_is_blocked_even_if_unit_name_matches() {
        let (composition, delivery) = valid_nutrient_fixture();
        let mut target = valid_nutrient_target();
        target.value_scale = 100;
        let assessment = assess_nutrient_target(&composition, &delivery, &target);
        assert_eq!(assessment.status, NutrientTargetStatus::Blocked);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientTargetBlockerCode::NutrientScaleMismatch })
        );
    }

    #[test]
    fn target_requires_scope_period_and_source_references() {
        let (composition, delivery) = valid_nutrient_fixture();
        let mut target = valid_nutrient_target();
        target.target_scope_ref.clear();
        target.period_ref.clear();
        target.source_ref.clear();
        let assessment = assess_nutrient_target(&composition, &delivery, &target);
        assert_eq!(assessment.status, NutrientTargetStatus::Blocked);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientTargetBlockerCode::EmptyTargetScope })
        );
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientTargetBlockerCode::EmptyTargetPeriod })
        );
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientTargetBlockerCode::EmptyTargetSource })
        );
    }

    #[test]
    fn nutrient_target_json_preserves_wide_values() {
        let (composition, delivery) = valid_nutrient_fixture();
        let mut target = valid_nutrient_target();
        target.target_amount_scaled = u64::MAX;
        target.value_scale = composition.value_scale;
        let serialized = serde_json::to_string(&target).expect("target serializes");
        assert!(serialized.contains("\"target_amount_scaled\":\"18446744073709551615\""));
        let decoded: NutrientSupplyTargetV1 =
            serde_json::from_str(&serialized).expect("target deserializes");
        assert_eq!(decoded, target);

        let assessment = assess_nutrient_target(&composition, &delivery, &target);
        let result_json = serde_json::to_string(&assessment).expect("assessment serializes");
        let decoded_assessment: NutrientTargetAssessmentV1 =
            serde_json::from_str(&result_json).expect("assessment deserializes");
        assert_eq!(decoded_assessment, assessment);
    }

    #[test]
    fn target_scope_mismatch_is_blocked() {
        let (composition, delivery) = valid_nutrient_fixture();
        let mut target = valid_nutrient_target();
        target.target_scope_ref = "scope:regional-001".to_owned();
        let assessment = assess_nutrient_target(&composition, &delivery, &target);
        assert_eq!(assessment.status, NutrientTargetStatus::Blocked);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientTargetBlockerCode::TargetScopeMismatch })
        );
        assert_eq!(assessment.supply_numerator, None);
    }

    #[test]
    fn target_period_mismatch_is_blocked() {
        let (composition, delivery) = valid_nutrient_fixture();
        let mut target = valid_nutrient_target();
        target.period_ref = "period:2026-12".to_owned();
        let assessment = assess_nutrient_target(&composition, &delivery, &target);
        assert_eq!(assessment.status, NutrientTargetStatus::Blocked);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientTargetBlockerCode::TargetPeriodMismatch })
        );
        assert_eq!(assessment.supply_numerator, None);
    }

    fn valid_supply_scenario() -> NutrientSupplyScenarioV1 {
        let (composition, delivery) = valid_nutrient_fixture();
        NutrientSupplyScenarioV1 {
            schema_version: GARDEN_RESOURCE_SCHEMA_VERSION,
            scenario_id: "SYNTH-FOOD-001".to_owned(),
            target_scope_ref: "scope:community-001".to_owned(),
            period_ref: "period:2026-11".to_owned(),
            lines: vec![NutrientSupplyLineV1 {
                composition,
                delivery,
            }],
        }
    }

    #[test]
    fn portfolio_aggregates_two_foods_with_exact_common_scale() {
        let mut scenario = valid_supply_scenario();
        let mut second = scenario.lines[0].clone();
        second.composition.food_record_id = "FOOD-002".to_owned();
        second.composition.amount_per_100g_scaled = Some(100_000);
        second.composition.source_ref = "fixture:composition-002".to_owned();
        second.delivery.food_record_id = "FOOD-002".to_owned();
        second.delivery.edible_delivered_grams = 100;
        second.delivery.delivery_source_ref = "fixture:delivered-edible-mass-002".to_owned();
        scenario.lines.push(second);

        let mut target = valid_nutrient_target();
        target.target_amount_scaled = 250_000;
        let assessment = evaluate_nutrient_supply_scenario(&scenario, &target);
        assert_eq!(
            assessment.status,
            NutrientPortfolioStatus::MeetsDeclaredMinimum
        );
        assert_eq!(assessment.total_edible_delivered_grams, Some(175));
        assert_eq!(assessment.supply_numerator, Some(28_750_000));
        assert_eq!(assessment.target_numerator, Some(25_000_000));
        assert_eq!(assessment.denominator, Some(100_000));
        assert_eq!(assessment.shortfall_numerator, Some(0));
        assert_eq!(assessment.composition_source_refs.len(), 2);
        assert_eq!(assessment.delivery_source_refs.len(), 2);
        assert_eq!(assessment.line_receipts.len(), 2);
        assert_eq!(assessment.line_receipts[0].edible_delivered_grams, 75);
        assert_eq!(assessment.line_receipts[1].food_record_id, "FOOD-002");

        let serialized = serde_json::to_string(&assessment).expect("portfolio serializes");
        let decoded: NutrientPortfolioAssessmentV1 =
            serde_json::from_str(&serialized).expect("portfolio deserializes");
        assert_eq!(decoded, assessment);
        assert!(serialized.contains("\"supply_numerator\":\"28750000\""));
    }

    #[test]
    fn portfolio_does_not_treat_one_missing_food_value_as_zero() {
        let mut scenario = valid_supply_scenario();
        let mut second = scenario.lines[0].clone();
        second.composition.food_record_id = "FOOD-002".to_owned();
        second.composition.amount_per_100g_scaled = None;
        second.delivery.food_record_id = "FOOD-002".to_owned();
        second.delivery.delivery_source_ref = "fixture:delivered-edible-mass-002".to_owned();
        scenario.lines.push(second);

        let assessment = evaluate_nutrient_supply_scenario(&scenario, &valid_nutrient_target());
        assert_eq!(assessment.status, NutrientPortfolioStatus::NotAssessable);
        assert_eq!(assessment.supply_numerator, None);
        assert_eq!(assessment.total_edible_delivered_grams, None);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientPortfolioBlockerCode::MissingCompositionValue })
        );
    }

    #[test]
    fn portfolio_rejects_duplicate_delivery_references() {
        let mut scenario = valid_supply_scenario();
        let mut second = scenario.lines[0].clone();
        second.composition.food_record_id = "FOOD-002".to_owned();
        second.delivery.food_record_id = "FOOD-002".to_owned();
        // Reusing the first line's receipt must not double-count its delivery.
        scenario.lines.push(second);
        let assessment = evaluate_nutrient_supply_scenario(&scenario, &valid_nutrient_target());
        assert_eq!(assessment.status, NutrientPortfolioStatus::Blocked);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientPortfolioBlockerCode::DuplicateDeliverySource })
        );
        assert_eq!(assessment.supply_numerator, None);
    }

    #[test]
    fn portfolio_rejects_cross_scope_or_period_lines() {
        let mut scenario = valid_supply_scenario();
        let mut second = scenario.lines[0].clone();
        second.composition.food_record_id = "FOOD-002".to_owned();
        second.delivery.food_record_id = "FOOD-002".to_owned();
        second.delivery.delivery_source_ref = "fixture:delivery-002".to_owned();
        second.delivery.delivery_scope_ref = "scope:other-community".to_owned();
        scenario.lines.push(second);
        let assessment = evaluate_nutrient_supply_scenario(&scenario, &valid_nutrient_target());
        assert_eq!(assessment.status, NutrientPortfolioStatus::Blocked);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientPortfolioBlockerCode::ScopeMismatch })
        );
        assert_eq!(assessment.supply_numerator, None);
    }

    #[test]
    fn portfolio_rejects_cross_period_delivery_rows() {
        let mut scenario = valid_supply_scenario();
        let mut second = scenario.lines[0].clone();
        second.composition.food_record_id = "FOOD-002".to_owned();
        second.delivery.food_record_id = "FOOD-002".to_owned();
        second.delivery.delivery_source_ref = "fixture:delivery-002".to_owned();
        second.delivery.period_ref = "period:2026-12".to_owned();
        scenario.lines.push(second);
        let assessment = evaluate_nutrient_supply_scenario(&scenario, &valid_nutrient_target());
        assert_eq!(assessment.status, NutrientPortfolioStatus::Blocked);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientPortfolioBlockerCode::PeriodMismatch })
        );
        assert_eq!(assessment.supply_numerator, None);
    }

    #[test]
    fn portfolio_rejects_arithmetic_overflow() {
        let mut scenario = valid_supply_scenario();
        scenario.lines[0].composition.amount_per_100g_scaled = Some(u64::MAX);
        scenario.lines[0].composition.value_scale = u64::MAX;
        scenario.lines[0].delivery.edible_delivered_grams = u64::MAX;
        scenario.lines[0].delivery.delivery_source_ref = "fixture:delivery-max-1".to_owned();
        let mut second = scenario.lines[0].clone();
        second.delivery.delivery_source_ref = "fixture:delivery-max-2".to_owned();
        scenario.lines.push(second);
        let mut target = valid_nutrient_target();
        target.target_amount_scaled = u64::MAX;
        target.value_scale = u64::MAX;
        let assessment = evaluate_nutrient_supply_scenario(&scenario, &target);
        assert_eq!(assessment.status, NutrientPortfolioStatus::Blocked);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientPortfolioBlockerCode::QuantityOverflow })
        );
        assert_eq!(assessment.supply_numerator, None);
    }

    #[test]
    fn empty_portfolio_is_not_assumed_to_be_zero_supply() {
        let mut scenario = valid_supply_scenario();
        scenario.lines.clear();
        let assessment = evaluate_nutrient_supply_scenario(&scenario, &valid_nutrient_target());
        assert_eq!(assessment.status, NutrientPortfolioStatus::NotAssessable);
        assert_eq!(assessment.supply_numerator, None);
        assert!(
            assessment
                .blockers
                .iter()
                .any(|item| { item.code == NutrientPortfolioBlockerCode::NoSupplyLines })
        );
    }
}

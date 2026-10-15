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

}

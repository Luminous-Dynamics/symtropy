// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small, deterministic accounting checker for non-clinical garden infrastructure.
//!
//! This crate checks an explicitly declared balance boundary. It is not a coupled
//! physical simulator, a farm-yield estimator, a water-quality evaluator, or a
//! manufacturing qualification tool. A consistent ledger means only that the
//! supplied arithmetic fits within a declared tolerance and boundary.

pub const GARDEN_RESOURCE_SCHEMA_VERSION: u16 = 1;

/// Typed units prevent the checker from comparing unlike resource dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    WaterLitres,
    ElectricalEnergyKwh,
    MaterialKilograms,
}

/// Source class is provenance metadata, not proof that the source is authentic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticAssumption,
    ExternalObservationReference,
    QualifiedDispositionReference,
    SimulationDerived,
}

/// All quantities must use the unit implied by resource_kind. This checker cannot
/// convert units or authenticate the source.
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceBalanceInputV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub resource_kind: ResourceKind,
    pub inflow: f64,
    pub generated_or_added: f64,
    pub outflow: f64,
    pub declared_loss: f64,
    pub observed_storage_delta: f64,
    pub tolerance: f64,
    pub evidence_class: EvidenceClass,
    pub source_ref: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BalanceBlockerCode {
    UnsupportedSchema,
    EmptyScenarioId,
    MissingSourceReference,
    NonFiniteQuantity,
    NegativeFlowQuantity,
    InvalidTolerance,
    ArithmeticOverflow,
    ObservedBalanceMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalanceBlocker {
    pub code: BalanceBlockerCode,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BalanceDisposition {
    /// Arithmetic is consistent for this declared boundary only.
    AccountingConsistentForDeclaredBoundary,
    Blocked,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResourceBalanceDecisionV1 {
    pub scenario_id: String,
    pub resource_kind: ResourceKind,
    pub disposition: BalanceDisposition,
    pub expected_storage_delta: Option<f64>,
    pub absolute_error: Option<f64>,
    pub blockers: Vec<BalanceBlocker>,
}

/// Independently recompute:
/// inflow + generated_or_added - outflow - declared_loss = storage_delta.
///
/// This never proves model fidelity, resource safety, horticultural suitability,
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

    let non_finite = [
        input.inflow,
        input.generated_or_added,
        input.outflow,
        input.declared_loss,
        input.observed_storage_delta,
        input.tolerance,
    ]
    .iter()
    .any(|value| !value.is_finite());
    if non_finite {
        push_blocker(
            &mut blockers,
            BalanceBlockerCode::NonFiniteQuantity,
            "all quantities must be finite",
        );
    }

    if [input.inflow, input.generated_or_added, input.outflow, input.declared_loss]
        .iter()
        .any(|value| *value < 0.0)
    {
        push_blocker(
            &mut blockers,
            BalanceBlockerCode::NegativeFlowQuantity,
            "flow and loss quantities must be non-negative",
        );
    }
    if input.tolerance < 0.0 || !input.tolerance.is_finite() {
        push_blocker(
            &mut blockers,
            BalanceBlockerCode::InvalidTolerance,
            "tolerance must be finite and non-negative",
        );
    }

    let mut expected_storage_delta = None;
    let mut absolute_error = None;
    if !non_finite
        && input.inflow >= 0.0
        && input.generated_or_added >= 0.0
        && input.outflow >= 0.0
        && input.declared_loss >= 0.0
        && input.tolerance >= 0.0
    {
        let expected =
            input.inflow + input.generated_or_added - input.outflow - input.declared_loss;
        if !expected.is_finite() {
            push_blocker(
                &mut blockers,
                BalanceBlockerCode::ArithmeticOverflow,
                "recomputed balance is not representable as a finite value",
            );
        } else {
            expected_storage_delta = Some(expected);
            let error = (expected - input.observed_storage_delta).abs();
            if !error.is_finite() {
                push_blocker(
                    &mut blockers,
                    BalanceBlockerCode::ArithmeticOverflow,
                    "balance error is not representable as a finite value",
                );
            } else {
                absolute_error = Some(error);
                if error > input.tolerance {
                    push_blocker(
                        &mut blockers,
                        BalanceBlockerCode::ObservedBalanceMismatch,
                        "observed storage delta exceeds the declared balance tolerance",
                    );
                }
            }
        }
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
        expected_storage_delta,
        absolute_error,
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
            resource_kind: ResourceKind::WaterLitres,
            inflow: 10.0,
            generated_or_added: 0.0,
            outflow: 4.0,
            declared_loss: 1.0,
            observed_storage_delta: 5.0,
            tolerance: 0.01,
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
        assert_eq!(decision.expected_storage_delta, Some(5.0));
        assert_eq!(decision.absolute_error, Some(0.0));
    }

    #[test]
    fn mismatched_observation_is_blocked() {
        let mut input = valid_water_input();
        input.observed_storage_delta = 4.5;
        let decision = evaluate_resource_balance(&input);
        assert_eq!(decision.disposition, BalanceDisposition::Blocked);
        assert!(decision.blockers.iter().any(|item| {
            item.code == BalanceBlockerCode::ObservedBalanceMismatch
        }));
    }

    #[test]
    fn negative_flows_are_rejected() {
        let mut input = valid_water_input();
        input.declared_loss = -1.0;
        let decision = evaluate_resource_balance(&input);
        assert!(decision.blockers.iter().any(|item| {
            item.code == BalanceBlockerCode::NegativeFlowQuantity
        }));
    }

    #[test]
    fn non_finite_values_are_rejected() {
        let mut input = valid_water_input();
        input.observed_storage_delta = f64::NAN;
        let decision = evaluate_resource_balance(&input);
        assert!(decision.blockers.iter().any(|item| {
            item.code == BalanceBlockerCode::NonFiniteQuantity
        }));
        assert_eq!(decision.disposition, BalanceDisposition::Blocked);
    }

    #[test]
    fn arithmetic_overflow_is_not_accepted_as_a_balance() {
        let mut input = valid_water_input();
        input.inflow = f64::MAX;
        input.generated_or_added = f64::MAX;
        let decision = evaluate_resource_balance(&input);
        assert!(decision.blockers.iter().any(|item| {
            item.code == BalanceBlockerCode::ArithmeticOverflow
        }));
        assert_eq!(decision.disposition, BalanceDisposition::Blocked);
    }

    #[test]
    fn missing_source_reference_is_blocked() {
        let mut input = valid_water_input();
        input.source_ref.clear();
        let decision = evaluate_resource_balance(&input);
        assert!(decision.blockers.iter().any(|item| {
            item.code == BalanceBlockerCode::MissingSourceReference
        }));
    }

    #[test]
    fn negative_or_non_finite_tolerance_is_blocked() {
        let mut negative = valid_water_input();
        negative.tolerance = -0.1;
        assert!(evaluate_resource_balance(&negative).blockers.iter().any(|item| {
            item.code == BalanceBlockerCode::InvalidTolerance
        }));

        let mut infinite = valid_water_input();
        infinite.tolerance = f64::INFINITY;
        assert!(evaluate_resource_balance(&infinite).blockers.iter().any(|item| {
            item.code == BalanceBlockerCode::NonFiniteQuantity
        }));
    }

    #[test]
    fn exact_zero_balance_can_use_zero_tolerance() {
        let mut input = valid_water_input();
        input.inflow = 1.0;
        input.outflow = 1.0;
        input.declared_loss = 0.0;
        input.observed_storage_delta = 0.0;
        input.tolerance = 0.0;
        let decision = evaluate_resource_balance(&input);
        assert_eq!(
            decision.disposition,
            BalanceDisposition::AccountingConsistentForDeclaredBoundary
        );
    }
}

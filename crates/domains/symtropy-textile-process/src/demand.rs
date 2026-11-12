// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Paired, demand-aware production strategy comparison.
//!
//! This module compares deterministic production decisions against the same
//! forecast, confirmed orders, realized demand, available accepted inventory,
//! and declared assumptions. It does not predict demand or prove lifecycle
//! benefits. Existing inventory use is a counterfactual allocation until an
//! observed stock-transfer/use receipt establishes that it happened.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub const ACCEPTANCE_RATE_DENOMINATOR: u64 = 10_000;
pub const MAX_DEMAND_SCENARIOS: usize = 128;

/// Production and fulfillment policies. Policy output remains a vector of
/// physical/service measures; it is never compressed into one sustainability
/// score.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ProductionPolicy {
    MadeToOrder,
    PreorderPlusBuffer,
    SmallBatchFromOrders,
    ForecastBatch,
    ReuseInventoryFirst,
}

pub const PRODUCTION_POLICIES: [ProductionPolicy; 5] = [
    ProductionPolicy::MadeToOrder,
    ProductionPolicy::PreorderPlusBuffer,
    ProductionPolicy::SmallBatchFromOrders,
    ProductionPolicy::ForecastBatch,
    ProductionPolicy::ReuseInventoryFirst,
];

/// One paired demand case. Resource values are per planned production unit,
/// in caller-declared units; no conversions or life-cycle impact factors occur.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemandScenario {
    pub id: String,
    pub fixture_ref: String,
    pub profile_id: String,
    pub functional_unit_id: String,
    pub forecast_units: u64,
    pub confirmed_order_units: u64,
    pub realized_demand_units: u64,
    pub existing_accepted_inventory_units: u64,
    pub batch_size_units: u64,
    pub safety_stock_units: u64,
    pub production_lead_ticks: u64,
    pub maximum_customer_wait_ticks: u64,
    /// Integer basis points in the closed interval [0, 10000].
    /// This is a declared scenario assumption/measurement, not a model output.
    /// Accepted units use conservative integer-floor arithmetic; small lots can
    /// be biased downward, so physical trials must use observed integer QA counts.
    pub acceptance_rate_basis_points: u64,
    pub energy_unit_id: String,
    pub energy_per_planned_unit: u64,
    pub water_unit_id: String,
    pub water_per_planned_unit: u64,
    pub material_unit_id: String,
    pub material_per_planned_unit: u64,
    pub forecast_evidence_ref: String,
    pub order_evidence_ref: String,
    pub realized_demand_evidence_ref: String,
    pub inventory_condition_evidence_ref: String,
    pub acceptance_rate_evidence_ref: String,
    pub process_intensity_evidence_ref: String,
}

/// Paired corpus: every policy receives identical inputs for each scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemandCorpus {
    pub id: String,
    pub version: String,
    pub synthetic_only: bool,
    pub interpretation_boundary: String,
    pub scenarios: Vec<DemandScenario>,
}

/// Outcome for one policy on one scenario. All quantities are integers and
/// preserve their declared meaning; unused stock is not silently called waste.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyDemandOutcome {
    pub policy: ProductionPolicy,
    pub planned_new_production_units: u64,
    pub accepted_new_output_units: u64,
    pub rejected_new_output_units: u64,
    pub existing_inventory_used_units: u64,
    pub unused_existing_inventory_units: u64,
    pub new_output_fulfilled_units: u64,
    pub total_fulfilled_units: u64,
    pub on_time_fulfilled_units: u64,
    pub late_fulfilled_units: u64,
    pub stockout_units: u64,
    pub unsold_new_output_units: u64,
    pub manufacturing_energy_units: u64,
    pub manufacturing_water_units: u64,
    pub manufacturing_material_units: u64,
    pub service_fill_rate_basis_points: Option<u64>,
    pub on_time_rate_basis_points: Option<u64>,
    /// Unsold new output per 10,000 fulfilled service units; None when no
    /// service unit was fulfilled (undefined, not zero).
    pub unsold_new_per_10_000_fulfilled_units: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioDemandResult {
    /// Full frozen input and evidence references, not just the derived outputs.
    pub scenario: DemandScenario,
    pub policy_outcomes: Vec<PolicyDemandOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemandCorpusReport {
    pub corpus_id: String,
    pub corpus_version: String,
    pub synthetic_only: bool,
    pub scenario_count: usize,
    /// Results remain stratified by scenario/profile/functional unit. The
    /// evaluator does not sum incompatible units across unrelated scenarios.
    pub scenario_results: Vec<ScenarioDemandResult>,
    pub interpretation_boundary: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DemandError {
    EmptyCorpusId,
    EmptyCorpusVersion,
    EmptyInterpretationBoundary,
    EmptyScenarioSet,
    TooManyScenarios { actual: usize, maximum: usize },
    EmptyScenarioId,
    DuplicateScenarioId { scenario_id: String },
    EmptyFixtureReference { scenario_id: String },
    EmptyProfileId { scenario_id: String },
    EmptyFunctionalUnitId { scenario_id: String },
    EmptyEnergyUnitId { scenario_id: String },
    EmptyWaterUnitId { scenario_id: String },
    EmptyMaterialUnitId { scenario_id: String },
    EmptyEvidenceReference { scenario_id: String, field: &'static str },
    ZeroBatchSize { scenario_id: String },
    AcceptanceRateOutOfRange { scenario_id: String, value: u64 },
    ArithmeticOverflow { scenario_id: String, field: &'static str },
}

impl fmt::Display for DemandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "textile demand evaluation error: {self:?}")
    }
}

impl Error for DemandError {}

/// Evaluate all declared policies on exactly the same scenarios. The result
/// order is canonicalized by scenario ID and policy declaration order.
pub fn evaluate_demand_corpus(
    corpus: &DemandCorpus,
) -> Result<DemandCorpusReport, DemandError> {
    if corpus.id.trim().is_empty() {
        return Err(DemandError::EmptyCorpusId);
    }
    if corpus.version.trim().is_empty() {
        return Err(DemandError::EmptyCorpusVersion);
    }
    if corpus.interpretation_boundary.trim().is_empty() {
        return Err(DemandError::EmptyInterpretationBoundary);
    }
    if corpus.scenarios.is_empty() {
        return Err(DemandError::EmptyScenarioSet);
    }
    if corpus.scenarios.len() > MAX_DEMAND_SCENARIOS {
        return Err(DemandError::TooManyScenarios {
            actual: corpus.scenarios.len(),
            maximum: MAX_DEMAND_SCENARIOS,
        });
    }

    let mut scenarios = corpus.scenarios.clone();
    scenarios.sort_by(|left, right| left.id.cmp(&right.id));
    let mut seen = BTreeSet::new();
    let mut scenario_results = Vec::with_capacity(scenarios.len());

    for scenario in &scenarios {
        validate_scenario(scenario)?;
        if !seen.insert(scenario.id.as_str()) {
            return Err(DemandError::DuplicateScenarioId {
                scenario_id: scenario.id.clone(),
            });
        }

        let mut policy_outcomes = Vec::with_capacity(PRODUCTION_POLICIES.len());
        for policy in PRODUCTION_POLICIES {
            policy_outcomes.push(evaluate_policy(scenario, policy)?);
        }
        scenario_results.push(ScenarioDemandResult {
            scenario: scenario.clone(),
            policy_outcomes,
        });
    }

    Ok(DemandCorpusReport {
        corpus_id: corpus.id.clone(),
        corpus_version: corpus.version.clone(),
        synthetic_only: corpus.synthetic_only,
        scenario_count: scenario_results.len(),
        scenario_results,
        interpretation_boundary: corpus.interpretation_boundary.clone(),
    })
}

fn validate_scenario(scenario: &DemandScenario) -> Result<(), DemandError> {
    if scenario.id.trim().is_empty() {
        return Err(DemandError::EmptyScenarioId);
    }
    if scenario.fixture_ref.trim().is_empty() {
        return Err(DemandError::EmptyFixtureReference {
            scenario_id: scenario.id.clone(),
        });
    }
    if scenario.profile_id.trim().is_empty() {
        return Err(DemandError::EmptyProfileId {
            scenario_id: scenario.id.clone(),
        });
    }
    if scenario.functional_unit_id.trim().is_empty() {
        return Err(DemandError::EmptyFunctionalUnitId {
            scenario_id: scenario.id.clone(),
        });
    }
    if scenario.energy_unit_id.trim().is_empty() {
        return Err(DemandError::EmptyEnergyUnitId {
            scenario_id: scenario.id.clone(),
        });
    }
    if scenario.water_unit_id.trim().is_empty() {
        return Err(DemandError::EmptyWaterUnitId {
            scenario_id: scenario.id.clone(),
        });
    }
    if scenario.material_unit_id.trim().is_empty() {
        return Err(DemandError::EmptyMaterialUnitId {
            scenario_id: scenario.id.clone(),
        });
    }
    if scenario.batch_size_units == 0 {
        return Err(DemandError::ZeroBatchSize {
            scenario_id: scenario.id.clone(),
        });
    }
    if scenario.acceptance_rate_basis_points > ACCEPTANCE_RATE_DENOMINATOR {
        return Err(DemandError::AcceptanceRateOutOfRange {
            scenario_id: scenario.id.clone(),
            value: scenario.acceptance_rate_basis_points,
        });
    }

    for (field, value) in [
        ("forecast_evidence_ref", scenario.forecast_evidence_ref.as_str()),
        ("order_evidence_ref", scenario.order_evidence_ref.as_str()),
        (
            "realized_demand_evidence_ref",
            scenario.realized_demand_evidence_ref.as_str(),
        ),
        (
            "inventory_condition_evidence_ref",
            scenario.inventory_condition_evidence_ref.as_str(),
        ),
        (
            "acceptance_rate_evidence_ref",
            scenario.acceptance_rate_evidence_ref.as_str(),
        ),
        (
            "process_intensity_evidence_ref",
            scenario.process_intensity_evidence_ref.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(DemandError::EmptyEvidenceReference {
                scenario_id: scenario.id.clone(),
                field,
            });
        }
    }
    Ok(())
}

fn evaluate_policy(
    scenario: &DemandScenario,
    policy: ProductionPolicy,
) -> Result<PolicyDemandOutcome, DemandError> {
    let planned = match policy {
        ProductionPolicy::MadeToOrder => scenario.confirmed_order_units,
        ProductionPolicy::PreorderPlusBuffer => checked_add(
            scenario.confirmed_order_units,
            scenario.safety_stock_units,
            &scenario.id,
            "preorder plus safety stock",
        )?,
        ProductionPolicy::SmallBatchFromOrders => {
            round_up_to_batch(scenario.confirmed_order_units, scenario.batch_size_units)
                .ok_or_else(|| DemandError::ArithmeticOverflow {
                    scenario_id: scenario.id.clone(),
                    field: "small-batch rounding",
                })?
        }
        ProductionPolicy::ForecastBatch => {
            // A forecast cannot override already-confirmed orders. Batch-round
            // the larger of the forecast and the committed order book.
            let signal = scenario
                .forecast_units
                .max(scenario.confirmed_order_units);
            round_up_to_batch(signal, scenario.batch_size_units).ok_or_else(|| {
                DemandError::ArithmeticOverflow {
                    scenario_id: scenario.id.clone(),
                    field: "forecast-batch rounding",
                }
            })?
        }
        ProductionPolicy::ReuseInventoryFirst => scenario
            .confirmed_order_units
            .saturating_sub(scenario.existing_accepted_inventory_units),
    };

    // Integer floor semantics are explicit: the assumed acceptance fraction
    // cannot create fractional physical units or more accepted units than made.
    let accepted_u128 = u128::from(planned)
        .checked_mul(u128::from(scenario.acceptance_rate_basis_points))
        .ok_or_else(|| DemandError::ArithmeticOverflow {
            scenario_id: scenario.id.clone(),
            field: "accepted output",
        })?
        / u128::from(ACCEPTANCE_RATE_DENOMINATOR);
    let accepted_new = u64::try_from(accepted_u128).map_err(|_| {
        DemandError::ArithmeticOverflow {
            scenario_id: scenario.id.clone(),
            field: "accepted output conversion",
        }
    })?;
    let rejected_new = planned.checked_sub(accepted_new).ok_or_else(|| {
        DemandError::ArithmeticOverflow {
            scenario_id: scenario.id.clone(),
            field: "rejected output partition",
        }
    })?;

    // Only the inventory-first policy consumes existing stock in this
    // counterfactual. Actual physical use/second-use requires a separate receipt.
    let existing_used = if policy == ProductionPolicy::ReuseInventoryFirst {
        scenario
            .existing_accepted_inventory_units
            .min(scenario.realized_demand_units)
    } else {
        0
    };
    let unused_existing = scenario
        .existing_accepted_inventory_units
        .checked_sub(existing_used)
        .ok_or_else(|| DemandError::ArithmeticOverflow {
            scenario_id: scenario.id.clone(),
            field: "existing inventory partition",
        })?;
    let remaining_demand = scenario
        .realized_demand_units
        .checked_sub(existing_used)
        .ok_or_else(|| DemandError::ArithmeticOverflow {
            scenario_id: scenario.id.clone(),
            field: "remaining demand",
        })?;
    let new_fulfilled = accepted_new.min(remaining_demand);
    let total_fulfilled = checked_add(
        existing_used,
        new_fulfilled,
        &scenario.id,
        "total fulfilled units",
    )?;
    let stockout = scenario
        .realized_demand_units
        .checked_sub(total_fulfilled)
        .ok_or_else(|| DemandError::ArithmeticOverflow {
            scenario_id: scenario.id.clone(),
            field: "stockout",
        })?;
    let unsold_new = accepted_new
        .checked_sub(new_fulfilled)
        .ok_or_else(|| DemandError::ArithmeticOverflow {
            scenario_id: scenario.id.clone(),
            field: "unsold new output",
        })?;

    let new_output_on_time = scenario.production_lead_ticks
        <= scenario.maximum_customer_wait_ticks;
    let on_time_new = if new_output_on_time {
        new_fulfilled
    } else {
        0
    };
    let late_new = new_fulfilled
        .checked_sub(on_time_new)
        .ok_or_else(|| DemandError::ArithmeticOverflow {
            scenario_id: scenario.id.clone(),
            field: "late fulfilled units",
        })?;
    let on_time_fulfilled = checked_add(
        existing_used,
        on_time_new,
        &scenario.id,
        "on-time fulfilled units",
    )?;
    let late_fulfilled = total_fulfilled
        .checked_sub(on_time_fulfilled)
        .ok_or_else(|| DemandError::ArithmeticOverflow {
            scenario_id: scenario.id.clone(),
            field: "late fulfilled units total",
        })?;

    let manufacturing_energy = checked_mul(
        planned,
        scenario.energy_per_planned_unit,
        &scenario.id,
        "manufacturing energy",
    )?;
    let manufacturing_water = checked_mul(
        planned,
        scenario.water_per_planned_unit,
        &scenario.id,
        "manufacturing water",
    )?;
    let manufacturing_material = checked_mul(
        planned,
        scenario.material_per_planned_unit,
        &scenario.id,
        "manufacturing material",
    )?;

    Ok(PolicyDemandOutcome {
        policy,
        planned_new_production_units: planned,
        accepted_new_output_units: accepted_new,
        rejected_new_output_units: rejected_new,
        existing_inventory_used_units: existing_used,
        unused_existing_inventory_units: unused_existing,
        new_output_fulfilled_units: new_fulfilled,
        total_fulfilled_units: total_fulfilled,
        on_time_fulfilled_units: on_time_fulfilled,
        late_fulfilled_units: late_fulfilled,
        stockout_units: stockout,
        unsold_new_output_units: unsold_new,
        manufacturing_energy_units: manufacturing_energy,
        manufacturing_water_units: manufacturing_water,
        manufacturing_material_units: manufacturing_material,
        service_fill_rate_basis_points: ratio_basis_points(
            total_fulfilled,
            scenario.realized_demand_units,
            &scenario.id,
            "service fill rate",
        )?,
        on_time_rate_basis_points: ratio_basis_points(
            on_time_fulfilled,
            scenario.realized_demand_units,
            &scenario.id,
            "on-time rate",
        )?,
        unsold_new_per_10_000_fulfilled_units: ratio_basis_points(
            unsold_new,
            total_fulfilled,
            &scenario.id,
            "unsold per fulfilled unit",
        )?,
    })
}

fn ratio_basis_points(
    numerator: u64,
    denominator: u64,
    scenario_id: &str,
    field: &'static str,
) -> Result<Option<u64>, DemandError> {
    if denominator == 0 {
        return Ok(None);
    }
    let scaled = u128::from(numerator)
        .checked_mul(u128::from(ACCEPTANCE_RATE_DENOMINATOR))
        .ok_or_else(|| DemandError::ArithmeticOverflow {
            scenario_id: scenario_id.to_owned(),
            field,
        })?
        / u128::from(denominator);
    let value = u64::try_from(scaled).map_err(|_| DemandError::ArithmeticOverflow {
        scenario_id: scenario_id.to_owned(),
        field,
    })?;
    Ok(Some(value))
}

fn round_up_to_batch(units: u64, batch_size: u64) -> Option<u64> {
    if units == 0 {
        return Some(0);
    }
    let full_batches = units / batch_size;
    let remainder = units % batch_size;
    let batches = if remainder == 0 {
        full_batches
    } else {
        full_batches.checked_add(1)?
    };
    batches.checked_mul(batch_size)
}

fn checked_add(
    left: u64,
    right: u64,
    scenario_id: &str,
    field: &'static str,
) -> Result<u64, DemandError> {
    left.checked_add(right)
        .ok_or_else(|| DemandError::ArithmeticOverflow {
            scenario_id: scenario_id.to_owned(),
            field,
        })
}

fn checked_mul(
    left: u64,
    right: u64,
    scenario_id: &str,
    field: &'static str,
) -> Result<u64, DemandError> {
    left.checked_mul(right)
        .ok_or_else(|| DemandError::ArithmeticOverflow {
            scenario_id: scenario_id.to_owned(),
            field,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario(id: &str) -> DemandScenario {
        DemandScenario {
            id: id.to_owned(),
            fixture_ref: format!("fixture:demand:{id}"),
            profile_id: "couture-decorative-v0".to_owned(),
            functional_unit_id: "event-service-v0".to_owned(),
            forecast_units: 12,
            confirmed_order_units: 5,
            realized_demand_units: 7,
            existing_accepted_inventory_units: 2,
            batch_size_units: 4,
            safety_stock_units: 2,
            production_lead_ticks: 3,
            maximum_customer_wait_ticks: 4,
            acceptance_rate_basis_points: 9_500,
            energy_unit_id: "energy-unit-v0".to_owned(),
            energy_per_planned_unit: 2,
            water_unit_id: "water-unit-v0".to_owned(),
            water_per_planned_unit: 1,
            material_unit_id: "material-unit-v0".to_owned(),
            material_per_planned_unit: 3,
            forecast_evidence_ref: format!("fixture:forecast:{id}"),
            order_evidence_ref: format!("fixture:orders:{id}"),
            realized_demand_evidence_ref: format!("fixture:realized:{id}"),
            inventory_condition_evidence_ref: format!("fixture:inventory:{id}"),
            acceptance_rate_evidence_ref: format!("fixture:acceptance:{id}"),
            process_intensity_evidence_ref: format!("fixture:intensity:{id}"),
        }
    }

    fn corpus(scenarios: Vec<DemandScenario>) -> DemandCorpus {
        DemandCorpus {
            id: "demand-campaign-v0".to_owned(),
            version: "1".to_owned(),
            synthetic_only: true,
            interpretation_boundary: concat!(
                "Synthetic paired comparison; ",
                "no probability or lifecycle benefit implied."
            )
            .to_owned(),
            scenarios,
        }
    }

    #[test]
    fn forecast_batch_exposes_surplus_while_reuse_policy_uses_existing_stock() {
        let report = evaluate_demand_corpus(&corpus(vec![scenario("formal-capsule")]))
            .expect("valid scenario should be evaluated");
        let results = &report.scenario_results[0].policy_outcomes;
        let find = |policy| results.iter().find(|value| value.policy == policy).unwrap();
        let made_to_order = find(ProductionPolicy::MadeToOrder);
        let forecast = find(ProductionPolicy::ForecastBatch);
        let reuse = find(ProductionPolicy::ReuseInventoryFirst);

        assert_eq!(made_to_order.planned_new_production_units, 5);
        assert_eq!(made_to_order.accepted_new_output_units, 4);
        assert_eq!(made_to_order.rejected_new_output_units, 1);
        assert_eq!(forecast.planned_new_production_units, 12);
        assert_eq!(forecast.accepted_new_output_units, 11);
        assert_eq!(forecast.unsold_new_output_units, 4);
        assert_eq!(forecast.manufacturing_energy_units, 24);
        assert_eq!(forecast.manufacturing_water_units, 12);
        assert_eq!(forecast.manufacturing_material_units, 36);

        assert_eq!(reuse.planned_new_production_units, 3);
        assert_eq!(reuse.existing_inventory_used_units, 2);
        assert_eq!(reuse.accepted_new_output_units, 2);
        assert_eq!(reuse.unsold_new_output_units, 0);
        assert_eq!(reuse.stockout_units, 3);
        assert_eq!(reuse.unused_existing_inventory_units, 0);
    }

    #[test]
    fn product_and_demand_ledgers_close_for_every_policy() {
        let report = evaluate_demand_corpus(&corpus(vec![scenario("ledger")])).unwrap();
        let scenario_result = &report.scenario_results[0];
        for outcome in &scenario_result.policy_outcomes {
            assert_eq!(
                outcome.planned_new_production_units,
                outcome.accepted_new_output_units + outcome.rejected_new_output_units
            );
            assert_eq!(
                outcome.accepted_new_output_units,
                outcome.new_output_fulfilled_units + outcome.unsold_new_output_units
            );
            assert_eq!(
                outcome.existing_inventory_used_units
                    + outcome.unused_existing_inventory_units,
                2
            );
            assert_eq!(
                outcome.total_fulfilled_units,
                outcome.on_time_fulfilled_units + outcome.late_fulfilled_units
            );
            assert_eq!(
                scenario_result.scenario.realized_demand_units,
                outcome.total_fulfilled_units + outcome.stockout_units
            );
            assert!(outcome.manufacturing_energy_units
                >= outcome.planned_new_production_units);
        }
    }

    #[test]
    fn strategy_decisions_do_not_use_realized_demand_as_the_forecast() {
        let first = scenario("paired-a");
        let mut second = scenario("paired-b");
        second.realized_demand_units = 20;
        second.realized_demand_evidence_ref = "fixture:realized:paired-b".to_owned();

        let a = evaluate_demand_corpus(&corpus(vec![first.clone()])).unwrap();
        let b = evaluate_demand_corpus(&corpus(vec![second.clone()])).unwrap();
        for policy in PRODUCTION_POLICIES {
            let produced_a = a.scenario_results[0]
                .policy_outcomes
                .iter()
                .find(|item| item.policy == policy)
                .unwrap()
                .planned_new_production_units;
            let produced_b = b.scenario_results[0]
                .policy_outcomes
                .iter()
                .find(|item| item.policy == policy)
                .unwrap()
                .planned_new_production_units;
            assert_eq!(produced_a, produced_b);
        }
    }

    #[test]
    fn scenario_input_order_does_not_change_normalized_report() {
        let a = scenario("z-formal");
        let b = scenario("a-workwear");
        let report_a = evaluate_demand_corpus(&corpus(vec![a.clone(), b.clone()])).unwrap();
        let report_b = evaluate_demand_corpus(&corpus(vec![b, a])).unwrap();
        assert_eq!(report_a, report_b);
    }

    #[test]
    fn invalid_acceptance_rate_and_batch_size_fail_closed() {
        let mut invalid_rate = scenario("invalid-rate");
        invalid_rate.acceptance_rate_basis_points = 10_001;
        assert!(matches!(
            evaluate_demand_corpus(&corpus(vec![invalid_rate])),
            Err(DemandError::AcceptanceRateOutOfRange { .. })
        ));

        let mut zero_batch = scenario("zero-batch");
        zero_batch.batch_size_units = 0;
        assert!(matches!(
            evaluate_demand_corpus(&corpus(vec![zero_batch])),
            Err(DemandError::ZeroBatchSize { .. })
        ));
    }

    #[test]
    fn arithmetic_overflow_fails_instead_of_wrapping() {
        let mut bad = scenario("overflow");
        bad.confirmed_order_units = u64::MAX;
        bad.realized_demand_units = 0;
        bad.safety_stock_units = 1;
        bad.energy_per_planned_unit = 0;
        bad.water_per_planned_unit = 0;
        bad.material_per_planned_unit = 0;
        assert!(matches!(
            evaluate_demand_corpus(&corpus(vec![bad])),
            Err(DemandError::ArithmeticOverflow {
                field: "preorder plus safety stock",
                ..
            })
        ));

        let mut bad_intensity = scenario("intensity-overflow");
        bad_intensity.confirmed_order_units = u64::MAX;
        bad_intensity.energy_per_planned_unit = 2;
        assert!(matches!(
            evaluate_demand_corpus(&corpus(vec![bad_intensity])),
            Err(DemandError::ArithmeticOverflow {
                field: "manufacturing energy",
                ..
            })
        ));
    }

    #[test]
    fn zero_demand_has_undefined_rates_not_divide_by_zero() {
        let mut no_demand = scenario("no-demand");
        no_demand.forecast_units = 0;
        no_demand.confirmed_order_units = 0;
        no_demand.realized_demand_units = 0;
        no_demand.safety_stock_units = 0;
        let report = evaluate_demand_corpus(&corpus(vec![no_demand])).unwrap();
        for outcome in &report.scenario_results[0].policy_outcomes {
            assert_eq!(outcome.service_fill_rate_basis_points, None);
            assert_eq!(outcome.on_time_rate_basis_points, None);
            assert_eq!(outcome.unsold_new_per_10_000_fulfilled_units, None);
        }
    }

    #[test]
    fn forecast_batch_never_plans_below_already_confirmed_orders() {
        let mut case = scenario("forecast-undercalls-orders");
        case.forecast_units = 2;
        case.confirmed_order_units = 9;
        case.batch_size_units = 4;
        let report = evaluate_demand_corpus(&corpus(vec![case])).unwrap();
        let forecast = report.scenario_results[0]
            .policy_outcomes
            .iter()
            .find(|item| item.policy == ProductionPolicy::ForecastBatch)
            .unwrap();
        assert_eq!(forecast.planned_new_production_units, 12);
        assert!(forecast.planned_new_production_units >= 9);
    }


    #[test]
    fn report_preserves_all_inputs_and_evidence_references_for_replay() {
        let input = scenario("audit-couture");
        let report = evaluate_demand_corpus(&corpus(vec![input.clone()])).unwrap();
        assert_eq!(report.scenario_results[0].scenario, input);
        assert_eq!(
            report.scenario_results[0].policy_outcomes.len(),
            PRODUCTION_POLICIES.len()
        );
    }

}

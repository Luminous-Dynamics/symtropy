//! Independent, test-only reference evaluator for a frozen mixed shipyard ledger.
//!
//! This intentionally uses linear scans and separate aggregation code rather than importing
//! any production aggregation helper. It is quadratic by design: fixtures are tiny, and the
//! implementation favors auditability over speed.

use shipyard_cost_trace::{
    AuditReport, EventKind, EvidencePermission, FinancialActualTotal, FinancialTransaction,
    LedgerEvent, QuantityDisposition, QuantityEvent, QuantityRate, QuantityReconciliation,
    RateCardCostTotal, Stage,
};

const CONFIG_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CONFIG_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn quantity(
    id: &str,
    build: &str,
    config: &str,
    task: &str,
    key: &str,
    unit: &str,
    stage: Stage,
    amount: f64,
    rate: Option<QuantityRate>,
) -> LedgerEvent {
    LedgerEvent {
        event_id: id.to_owned(),
        build_id: build.to_owned(),
        configuration_sha256: config.to_owned(),
        task_id: task.to_owned(),
        stage,
        source_ref: format!("fixture:{id}"),
        source_permission: EvidencePermission::PublicOrPermitted,
        source_permission_ref: "fixture-rights:public-test-data-v1".to_owned(),
        related_event_ids: Vec::new(),
        kind: EventKind::Quantity(QuantityEvent {
            quantity_key: key.to_owned(),
            quantity: amount,
            unit: unit.to_owned(),
            rate,
        }),
    }
}

fn financial(
    id: &str,
    build: &str,
    config: &str,
    task: &str,
    transaction_ref: &str,
    amount_minor_units: i64,
) -> LedgerEvent {
    LedgerEvent {
        event_id: id.to_owned(),
        build_id: build.to_owned(),
        configuration_sha256: config.to_owned(),
        task_id: task.to_owned(),
        stage: Stage::Observed,
        source_ref: format!("fixture:{transaction_ref}"),
        source_permission: EvidencePermission::PublicOrPermitted,
        source_permission_ref: "fixture-rights:public-test-data-v1".to_owned(),
        related_event_ids: Vec::new(),
        kind: EventKind::FinancialActual(FinancialTransaction {
            transaction_ref: transaction_ref.to_owned(),
            amount_minor_units,
            currency_basis_id: "ZAR-cent-2026-SA".to_owned(),
            minor_unit_exponent: 2,
        }),
    }
}

fn rate(unit_rate: f64, name: &str) -> QuantityRate {
    QuantityRate {
        unit_rate,
        currency_basis_id: "ZAR-cent-2026-SA".to_owned(),
        minor_unit_exponent: 2,
        rate_card_ref: name.to_owned(),
    }
}

fn frozen_fixture() -> Vec<LedgerEvent> {
    vec![
        quantity("weld-plan", "hull-001", CONFIG_A, "block-01-weld", "labor.welder.hours", "hour", Stage::Planned, 10.0, Some(rate(100.0, "labor-v1"))),
        quantity("weld-sim", "hull-001", CONFIG_A, "block-01-weld", "labor.welder.hours", "hour", Stage::Simulated, 11.0, Some(rate(100.0, "labor-v1"))),
        quantity("weld-actual-a", "hull-001", CONFIG_A, "block-01-weld", "labor.welder.hours", "hour", Stage::Observed, 7.0, Some(rate(100.0, "labor-v1"))),
        quantity("weld-actual-b", "hull-001", CONFIG_A, "block-01-weld", "labor.welder.hours", "hour", Stage::Observed, 5.0, Some(rate(100.0, "labor-v1"))),
        quantity("overtime", "hull-001", CONFIG_A, "block-01-weld", "labor.welder.overtime.hours", "hour", Stage::Observed, 2.0, Some(rate(150.0, "overtime-v1"))),
        quantity("aluminium-plan", "hull-001", CONFIG_A, "block-01-weld", "material.aluminium.issued.kg", "kg", Stage::Planned, 120.0, Some(rate(250.0, "material-v4"))),
        quantity("aluminium-issued", "hull-001", CONFIG_A, "block-01-weld", "material.aluminium.issued.kg", "kg", Stage::Observed, 125.0, Some(rate(250.0, "material-v4"))),
        quantity("aluminium-scrap", "hull-001", CONFIG_A, "block-01-weld", "material.aluminium.scrapped.kg", "kg", Stage::Observed, 3.0, None),
        quantity("rework", "hull-001", CONFIG_A, "block-01-weld", "labor.rework.hours", "hour", Stage::Observed, 3.0, Some(rate(100.0, "labor-v1"))),
        quantity("paint-plan", "hull-001", CONFIG_A, "block-02-paint", "labor.painter.hours", "hour", Stage::Planned, 10.0, Some(rate(80.0, "labor-v1"))),
        quantity("paint-actual", "hull-001", CONFIG_A, "block-02-paint", "labor.painter.hours", "hour", Stage::Observed, 8.0, Some(rate(80.0, "labor-v1"))),
        quantity("cnc-simulation-only", "hull-001", CONFIG_A, "block-03-cnc", "labor.cnc.hours", "hour", Stage::Simulated, 5.5, None),
        financial("invoice-weld", "hull-001", CONFIG_A, "block-01-weld", "invoice-800:line-01", 150_000),
        financial("credit-weld", "hull-001", CONFIG_A, "block-01-weld", "credit-800:line-01", -5_000),
        financial("invoice-paint", "hull-001", CONFIG_A, "block-02-paint", "invoice-801:line-02", 80_000),
        financial("invoice-revised-config", "hull-002", CONFIG_B, "block-01-weld", "invoice-900:line-01", 2_345),
    ]
}

/// Deliberately simple reference algorithm. It does not invoke audit or production helpers.
fn reference_evaluate(events: &[LedgerEvent]) -> AuditReport {
    let mut ordered: Vec<&LedgerEvent> = events.iter().collect();
    ordered.sort_by(|left, right| left.event_id.cmp(&right.event_id));

    let mut quantities: Vec<QuantityReconciliation> = Vec::new();
    let mut rate_costs: Vec<RateCardCostTotal> = Vec::new();
    let mut actuals: Vec<FinancialActualTotal> = Vec::new();

    for event in ordered {
        match &event.kind {
            EventKind::Quantity(item) => {
                let row_index = quantities.iter().position(|row| {
                    row.build_id == event.build_id
                        && row.configuration_sha256 == event.configuration_sha256
                        && row.task_id == event.task_id
                        && row.quantity_key == item.quantity_key
                        && row.unit == item.unit
                });
                let index = match row_index {
                    Some(index) => index,
                    None => {
                        quantities.push(QuantityReconciliation {
                            build_id: event.build_id.clone(),
                            configuration_sha256: event.configuration_sha256.clone(),
                            task_id: event.task_id.clone(),
                            quantity_key: item.quantity_key.clone(),
                            unit: item.unit.clone(),
                            planned: None,
                            simulated: None,
                            observed: None,
                            observed_minus_planned: None,
                            disposition: QuantityDisposition::MissingPlanAndObserved,
                        });
                        quantities.len() - 1
                    }
                };
                let target = &mut quantities[index];
                let bucket = match event.stage {
                    Stage::Planned => &mut target.planned,
                    Stage::Simulated => &mut target.simulated,
                    Stage::Observed => &mut target.observed,
                };
                *bucket = Some(bucket.unwrap_or(0.0) + item.quantity);
                if let Some(rate_card) = &item.rate {
                    let cost_index = rate_costs.iter().position(|row| {
                        row.build_id == event.build_id
                            && row.configuration_sha256 == event.configuration_sha256
                            && row.task_id == event.task_id
                            && row.quantity_key == item.quantity_key
                            && row.unit == item.unit
                            && row.stage == event.stage
                            && row.currency_basis_id == rate_card.currency_basis_id
                            && row.minor_unit_exponent == rate_card.minor_unit_exponent
                            && row.rate_card_ref == rate_card.rate_card_ref
                    });
                    let cost = item.quantity * rate_card.unit_rate;
                    if let Some(index) = cost_index {
                        rate_costs[index].amount += cost;
                    } else {
                        rate_costs.push(RateCardCostTotal {
                            build_id: event.build_id.clone(),
                            configuration_sha256: event.configuration_sha256.clone(),
                            task_id: event.task_id.clone(),
                            quantity_key: item.quantity_key.clone(),
                            unit: item.unit.clone(),
                            stage: event.stage,
                            currency_basis_id: rate_card.currency_basis_id.clone(),
                            minor_unit_exponent: rate_card.minor_unit_exponent,
                            rate_card_ref: rate_card.rate_card_ref.clone(),
                            amount: cost,
                        });
                    }
                }
            }
            EventKind::FinancialActual(transaction) => {
                let actual_index = actuals.iter().position(|row| {
                    row.build_id == event.build_id
                        && row.configuration_sha256 == event.configuration_sha256
                        && row.task_id == event.task_id
                        && row.currency_basis_id == transaction.currency_basis_id
                        && row.minor_unit_exponent == transaction.minor_unit_exponent
                });
                if let Some(index) = actual_index {
                    actuals[index].amount_minor_units += transaction.amount_minor_units;
                } else {
                    actuals.push(FinancialActualTotal {
                        build_id: event.build_id.clone(),
                        configuration_sha256: event.configuration_sha256.clone(),
                        task_id: event.task_id.clone(),
                        currency_basis_id: transaction.currency_basis_id.clone(),
                        minor_unit_exponent: transaction.minor_unit_exponent,
                        amount_minor_units: transaction.amount_minor_units,
                    });
                }
            }
        }
    }

    for row in &mut quantities {
        row.observed_minus_planned = match (row.observed, row.planned) {
            (Some(actual), Some(plan)) => Some(actual - plan),
            _ => None,
        };
        row.disposition = match (row.planned, row.observed) {
            (Some(_), Some(_)) => QuantityDisposition::Comparable,
            (Some(_), None) => QuantityDisposition::MissingObserved,
            (None, Some(_)) => QuantityDisposition::MissingPlan,
            (None, None) => QuantityDisposition::MissingPlanAndObserved,
        };
    }

    quantities.sort_by(|a, b| {
        (&a.build_id, &a.configuration_sha256, &a.task_id, &a.quantity_key, &a.unit)
            .cmp(&(&b.build_id, &b.configuration_sha256, &b.task_id, &b.quantity_key, &b.unit))
    });
    rate_costs.sort_by(|a, b| {
        (
            &a.build_id,
            &a.configuration_sha256,
            &a.task_id,
            &a.quantity_key,
            &a.unit,
            a.stage,
            &a.currency_basis_id,
            a.minor_unit_exponent,
            &a.rate_card_ref,
        )
            .cmp(&(
                &b.build_id,
                &b.configuration_sha256,
                &b.task_id,
                &b.quantity_key,
                &b.unit,
                b.stage,
                &b.currency_basis_id,
                b.minor_unit_exponent,
                &b.rate_card_ref,
            ))
    });
    actuals.sort_by(|a, b| {
        (
            &a.build_id,
            &a.configuration_sha256,
            &a.task_id,
            &a.currency_basis_id,
            a.minor_unit_exponent,
        )
            .cmp(&(
                &b.build_id,
                &b.configuration_sha256,
                &b.task_id,
                &b.currency_basis_id,
                b.minor_unit_exponent,
            ))
    });

    AuditReport {
        event_count: events.len(),
        quantity_reconciliations: quantities,
        rate_card_costs: rate_costs,
        financial_actuals: actuals,
    }
}

#[test]
fn independent_reference_evaluator_matches_frozen_mixed_shipyard_ledger() {
    let events = frozen_fixture();
    let expected = reference_evaluate(&events);
    let actual = shipyard_cost_trace::audit(&events).expect("valid frozen ledger");
    assert_eq!(actual, expected);

    let welding = actual
        .quantity_reconciliations
        .iter()
        .find(|row| row.quantity_key == "labor.welder.hours")
        .expect("welding hours");
    assert_eq!(welding.planned, Some(10.0));
    assert_eq!(welding.simulated, Some(11.0));
    assert_eq!(welding.observed, Some(12.0));
    assert_eq!(welding.observed_minus_planned, Some(2.0));

    let overtime = actual
        .rate_card_costs
        .iter()
        .find(|row| row.quantity_key == "labor.welder.overtime.hours")
        .expect("overtime estimate");
    assert_eq!(overtime.amount, 300.0);

    let aluminium = actual
        .quantity_reconciliations
        .iter()
        .find(|row| row.quantity_key == "material.aluminium.issued.kg")
        .expect("aluminium issued");
    assert_eq!(aluminium.planned, Some(120.0));
    assert_eq!(aluminium.observed, Some(125.0));
    assert_eq!(aluminium.observed_minus_planned, Some(5.0));
    let aluminium_actual_rate = actual
        .rate_card_costs
        .iter()
        .find(|row| {
            row.quantity_key == "material.aluminium.issued.kg"
                && row.stage == Stage::Observed
        })
        .expect("observed aluminium issue estimate");
    assert_eq!(aluminium_actual_rate.amount, 31_250.0);

    let rework = actual
        .rate_card_costs
        .iter()
        .find(|row| row.quantity_key == "labor.rework.hours")
        .expect("rework estimate");
    assert_eq!(rework.amount, 300.0);
    assert_eq!(actual.rate_card_costs.len(), 9);

    let paint = actual
        .quantity_reconciliations
        .iter()
        .find(|row| row.quantity_key == "labor.painter.hours")
        .expect("painting hours");
    assert_eq!(paint.planned, Some(10.0));
    assert_eq!(paint.observed, Some(8.0));
    assert_eq!(paint.observed_minus_planned, Some(-2.0));

    let simulation_only = actual
        .quantity_reconciliations
        .iter()
        .find(|row| row.quantity_key == "labor.cnc.hours")
        .expect("simulation-only CNC");
    assert_eq!(simulation_only.simulated, Some(5.5));
    assert_eq!(simulation_only.observed, None);
    assert_eq!(simulation_only.observed_minus_planned, None);

    let welding_actual = actual
        .financial_actuals
        .iter()
        .find(|row| row.build_id == "hull-001" && row.task_id == "block-01-weld")
        .expect("welding invoice less credit");
    assert_eq!(welding_actual.amount_minor_units, 145_000);
    assert_eq!(welding_actual.currency_basis_id, "ZAR-cent-2026-SA");
    assert_eq!(welding_actual.minor_unit_exponent, 2);

    let other_configuration = actual
        .financial_actuals
        .iter()
        .find(|row| row.build_id == "hull-002")
        .expect("other vessel/configuration financial actual");
    assert_eq!(other_configuration.amount_minor_units, 2_345);
    assert_eq!(actual.financial_actuals.len(), 3);
}

#[test]
fn independent_reference_evaluator_is_order_invariant() {
    let events = frozen_fixture();
    let expected = reference_evaluate(&events);
    let mut reversed = events.clone();
    reversed.reverse();
    assert_eq!(expected, reference_evaluate(&reversed));
    assert_eq!(
        shipyard_cost_trace::audit(&events).expect("forward ledger"),
        shipyard_cost_trace::audit(&reversed).expect("reversed ledger")
    );
}

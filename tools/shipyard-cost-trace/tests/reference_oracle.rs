//! Independent, test-only reference evaluator for a frozen mixed shipyard ledger.
//!
//! This intentionally uses linear scans and separate aggregation code rather than importing
//! any production aggregation helper. It is quadratic by design: fixtures are tiny, and the
//! implementation favors auditability over speed.

use shipyard_cost_trace::{
    AuditReport, EventKind, EvidencePermission, FinancialActualTotal, FinancialTransaction,
    LedgerEvent, QuantityAggregationSemantics, QuantityDisposition, QuantityEvent, QuantityRate,
    QuantityReconciliation, RateCardCostTotal, Stage,
};

const GOLDEN_TSV: &str = include_str!("fixtures/golden-ledger-v0.tsv");
const EXPECTED_HEADER: &str = "event_id\tbuild_id\tconfiguration_sha256\ttask_id\tstage\tkind\tquantity_key\tunit\tquantity\trate_minor_units\tcurrency_basis_id\tminor_unit_exponent\trate_card_ref\ttransaction_ref\tamount_minor_units\tsource_ref\tsource_permission_ref\trelated_event_ids";

fn parse_stage(value: &str) -> Stage {
    match value {
        "planned" => Stage::Planned,
        "simulated" => Stage::Simulated,
        "observed" => Stage::Observed,
        other => panic!("unknown stage in frozen fixture: {other}"),
    }
}

fn frozen_fixture() -> Vec<LedgerEvent> {
    let mut lines = GOLDEN_TSV.lines();
    assert_eq!(lines.next(), Some(EXPECTED_HEADER), "frozen corpus header");

    lines
        .filter(|line| !line.is_empty())
        .enumerate()
        .map(|(line_index, line)| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                fields.len(),
                18,
                "frozen corpus row {} must have 18 columns",
                line_index + 2
            );

            let stage = parse_stage(fields[4]);
            let related_event_ids = if fields[17] == "-" {
                Vec::new()
            } else {
                fields[17].split(';').map(str::to_owned).collect()
            };

            let kind = match fields[5] {
                "quantity" => {
                    let rate = if fields[9].is_empty() {
                        None
                    } else {
                        Some(QuantityRate {
                            unit_rate: fields[9].parse().expect("valid frozen rate-card unit rate"),
                            currency_basis_id: fields[10].to_owned(),
                            minor_unit_exponent: fields[11]
                                .parse()
                                .expect("valid frozen rate denomination"),
                            rate_card_ref: fields[12].to_owned(),
                        })
                    };
                    EventKind::Quantity(QuantityEvent {
                        quantity_key: fields[6].to_owned(),
                        unit: fields[7].to_owned(),
                        quantity: fields[8].parse().expect("valid frozen quantity"),
                        aggregation_semantics: QuantityAggregationSemantics::AdditiveContribution,
                        rate,
                    })
                }
                "financial" => EventKind::FinancialActual(FinancialTransaction {
                    transaction_ref: fields[13].to_owned(),
                    amount_minor_units: fields[14].parse().expect("valid frozen financial amount"),
                    currency_basis_id: fields[10].to_owned(),
                    minor_unit_exponent: fields[11]
                        .parse()
                        .expect("valid frozen financial denomination"),
                }),
                other => panic!("unknown frozen event kind: {other}"),
            };

            LedgerEvent {
                event_id: fields[0].to_owned(),
                build_id: fields[1].to_owned(),
                configuration_sha256: fields[2].to_owned(),
                task_id: fields[3].to_owned(),
                stage,
                kind,
                source_ref: fields[15].to_owned(),
                source_permission: EvidencePermission::PublicOrPermitted,
                source_permission_ref: fields[16].to_owned(),
                related_event_ids,
            }
        })
        .collect()
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
                let previous = (*bucket).unwrap_or(0.0);
                *bucket = Some(previous + item.quantity);
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
        (
            &a.build_id,
            &a.configuration_sha256,
            &a.task_id,
            &a.quantity_key,
            &a.unit,
        )
            .cmp(&(
                &b.build_id,
                &b.configuration_sha256,
                &b.task_id,
                &b.quantity_key,
                &b.unit,
            ))
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
fn frozen_corpus_shape_and_event_identity_are_pinned() {
    let events = frozen_fixture();
    let mut event_ids: Vec<&str> = events.iter().map(|event| event.event_id.as_str()).collect();
    event_ids.sort_unstable();
    event_ids.dedup();

    assert_eq!(events.len(), 16);
    assert_eq!(event_ids.len(), 16);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(&event.kind, EventKind::Quantity(_)))
            .count(),
        12
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(&event.kind, EventKind::FinancialActual(_)))
            .count(),
        4
    );
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
            row.quantity_key == "material.aluminium.issued.kg" && row.stage == Stage::Observed
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

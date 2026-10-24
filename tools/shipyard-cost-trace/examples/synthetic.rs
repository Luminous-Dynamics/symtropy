use shipyard_cost_trace::{
    EventKind, EvidencePermission, FinancialTransaction, LedgerEvent, QuantityAggregationSemantics,
    QuantityEvent, QuantityRate, Stage, audit,
};

const CONFIG: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn main() {
    let events = vec![
        LedgerEvent {
            event_id: "planned-weld-hours".to_owned(),
            build_id: "demo-utility-craft-001".to_owned(),
            configuration_sha256: CONFIG.to_owned(),
            task_id: "block-a-weld".to_owned(),
            stage: Stage::Planned,
            source_ref: "fixture:planned-hours".to_owned(),
            source_permission: EvidencePermission::PublicOrPermitted,
            source_permission_ref: "fixture-rights:public-test-data".to_owned(),
            related_event_ids: vec![],
            kind: EventKind::Quantity(QuantityEvent {
                quantity_key: "labor.welder.hours".to_owned(),
                quantity: 10.0,
                aggregation_semantics: QuantityAggregationSemantics::AdditiveContribution,
                unit: "hour".to_owned(),
                rate: Some(QuantityRate {
                    unit_rate: 50_000.0,
                    currency_basis_id: "ZAR-cent-2026-SA-demo".to_owned(),
                    minor_unit_exponent: 2,
                    rate_card_ref: "fixture:rate-card-v1".to_owned(),
                }),
            }),
        },
        LedgerEvent {
            event_id: "observed-weld-hours".to_owned(),
            build_id: "demo-utility-craft-001".to_owned(),
            configuration_sha256: CONFIG.to_owned(),
            task_id: "block-a-weld".to_owned(),
            stage: Stage::Observed,
            source_ref: "fixture:observed-hours".to_owned(),
            source_permission: EvidencePermission::PublicOrPermitted,
            source_permission_ref: "fixture-rights:public-test-data".to_owned(),
            related_event_ids: vec!["planned-weld-hours".to_owned()],
            kind: EventKind::Quantity(QuantityEvent {
                quantity_key: "labor.welder.hours".to_owned(),
                quantity: 12.0,
                aggregation_semantics: QuantityAggregationSemantics::AdditiveContribution,
                unit: "hour".to_owned(),
                rate: Some(QuantityRate {
                    unit_rate: 50_000.0,
                    currency_basis_id: "ZAR-cent-2026-SA-demo".to_owned(),
                    minor_unit_exponent: 2,
                    rate_card_ref: "fixture:rate-card-v1".to_owned(),
                }),
            }),
        },
        LedgerEvent {
            event_id: "invoice-line-1".to_owned(),
            build_id: "demo-utility-craft-001".to_owned(),
            configuration_sha256: CONFIG.to_owned(),
            task_id: "block-a-weld".to_owned(),
            stage: Stage::Observed,
            source_ref: "fixture:invoice-0001".to_owned(),
            source_permission: EvidencePermission::PublicOrPermitted,
            source_permission_ref: "fixture-rights:public-test-data".to_owned(),
            related_event_ids: vec!["observed-weld-hours".to_owned()],
            kind: EventKind::FinancialActual(FinancialTransaction {
                transaction_ref: "invoice-0001:line-1".to_owned(),
                amount_minor_units: 680_000,
                currency_basis_id: "ZAR-cent-2026-SA-demo".to_owned(),
                minor_unit_exponent: 2,
            }),
        },
    ];

    let report = audit(&events).expect("valid synthetic fixture");
    println!("schema,event_count,quantity_axes,rate_card_cost_rows,financial_actual_rows");
    println!(
        "shipyard-cost-trace-v0,{},{},{},{}",
        report.event_count,
        report.quantity_reconciliations.len(),
        report.rate_card_costs.len(),
        report.financial_actuals.len()
    );
    for row in &report.quantity_reconciliations {
        println!(
            "quantity_variance,{},{},{},{:?}",
            row.task_id, row.quantity_key, row.unit, row.observed_minus_planned
        );
    }
    println!("Synthetic accounting fixture only; not a shipyard cost estimate or savings claim.");
}

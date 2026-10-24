use shipyard_cost_trace::{
    EventKind, EvidencePermission, FinancialTransaction, LedgerEvent, QuantityAggregationSemantics,
    QuantityEvent, QuantityRate, Stage, audit,
};

const CONFIG: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn quantity_event(
    event_id: &str,
    stage: Stage,
    quantity: f64,
    rate: Option<QuantityRate>,
) -> LedgerEvent {
    LedgerEvent {
        event_id: event_id.to_owned(),
        build_id: "oracle-build".to_owned(),
        configuration_sha256: CONFIG.to_owned(),
        task_id: "fabrication-1".to_owned(),
        stage,
        source_ref: format!("fixture:{event_id}"),
        source_permission: EvidencePermission::PublicOrPermitted,
        source_permission_ref: "fixture-rights:public-test-data".to_owned(),
        related_event_ids: Vec::new(),
        kind: EventKind::Quantity(QuantityEvent {
            quantity_key: "labor.fitter.hours".to_owned(),
            quantity,
            aggregation_semantics: QuantityAggregationSemantics::AdditiveContribution,
            unit: "hour".to_owned(),
            rate,
        }),
    }
}

fn invoice_event(amount_minor_units: i64) -> LedgerEvent {
    LedgerEvent {
        event_id: "invoice-1".to_owned(),
        build_id: "oracle-build".to_owned(),
        configuration_sha256: CONFIG.to_owned(),
        task_id: "fabrication-1".to_owned(),
        stage: Stage::Observed,
        source_ref: "fixture:invoice-1".to_owned(),
        source_permission: EvidencePermission::PublicOrPermitted,
        source_permission_ref: "fixture-rights:public-test-data".to_owned(),
        related_event_ids: Vec::new(),
        kind: EventKind::FinancialActual(FinancialTransaction {
            transaction_ref: "invoice-1:line-1".to_owned(),
            amount_minor_units,
            currency_basis_id: "ZAR-cent-2026-SA".to_owned(),
            minor_unit_exponent: 2,
        }),
    }
}

fn test_rate(rate: f64) -> QuantityRate {
    QuantityRate {
        unit_rate: rate,
        currency_basis_id: "ZAR-cent-2026-SA".to_owned(),
        minor_unit_exponent: 2,
        rate_card_ref: "oracle-rate-card-v1".to_owned(),
    }
}

/// Tiny independent oracle for a single quantity axis:
/// the correct variance is observed minus planned, and only exists when both are present.
/// This loop exercises all 16 pairs in the declared 0..=3 finite integer fixture set.
#[test]
fn exhaustive_tiny_quantity_pairs_match_independent_arithmetic_oracle() {
    for planned in 0..=3 {
        for observed in 0..=3 {
            let events = vec![
                quantity_event("planned", Stage::Planned, f64::from(planned), None),
                quantity_event("observed", Stage::Observed, f64::from(observed), None),
            ];
            let report = audit(&events).expect("valid tiny oracle ledger");
            assert_eq!(report.quantity_reconciliations.len(), 1);
            let row = &report.quantity_reconciliations[0];
            assert_eq!(row.planned, Some(f64::from(planned)));
            assert_eq!(row.observed, Some(f64::from(observed)));
            assert_eq!(
                row.observed_minus_planned,
                Some(f64::from(observed) - f64::from(planned))
            );
        }
    }
}

/// Direct financial actual and rate-card estimate are independent measurements of different
/// accounting objects. Neither may be summed into one result without an explicit allocation.
#[test]
fn independent_cost_view_oracle_keeps_rate_estimate_and_invoice_separate() {
    let events = vec![
        quantity_event("planned-hours", Stage::Planned, 5.0, Some(test_rate(100.0))),
        quantity_event(
            "observed-hours",
            Stage::Observed,
            6.0,
            Some(test_rate(100.0)),
        ),
        invoice_event(72_500),
    ];
    let report = audit(&events).expect("valid mixed ledger");
    let planned_rate_estimate = report
        .rate_card_costs
        .iter()
        .find(|row| row.stage == Stage::Planned)
        .expect("planned rate estimate");
    let observed_rate_estimate = report
        .rate_card_costs
        .iter()
        .find(|row| row.stage == Stage::Observed)
        .expect("observed quantity rate estimate");

    assert_eq!(planned_rate_estimate.amount, 500.0);
    assert_eq!(observed_rate_estimate.amount, 600.0);
    assert_eq!(report.financial_actuals.len(), 1);
    assert_eq!(report.financial_actuals[0].amount_minor_units, 72_500);
    // The estimates and direct financial actual are intentionally separate output views.
    // Assert each view independently; do not combine them into a synthetic grand total.
}

/// Missing a plan is not the same as planning zero; the audit must preserve that distinction.
#[test]
fn independent_missingness_oracle_never_invents_zero_plan() {
    let events = vec![quantity_event("observed-only", Stage::Observed, 2.0, None)];
    let report = audit(&events).expect("valid observed-only ledger");
    let row = &report.quantity_reconciliations[0];
    assert_eq!(row.planned, None);
    assert_eq!(row.observed, Some(2.0));
    assert_eq!(row.observed_minus_planned, None);
}

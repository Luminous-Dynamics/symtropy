//! Evidence-bounded reconciliation of shipyard quantity events and financial actuals.
//!
//! This crate is an accounting/reconciliation kernel, not an ERP integration, scheduler,
//! engineering approval mechanism, or source-authentication service. It deliberately keeps
//! planned, simulated, and observed quantities separate from invoice/payroll financial actuals.
//! Rate-card estimates remain separate from direct financial actuals and must not simply be added.
//!
//! Quantity records accepted by [`audit`] must already represent additive contributions. Do not
//! feed cumulative snapshots directly to this reducer: use [`event_semantics::normalize_quantities`]
//! with an explicit bitemporal cutoff first. Corrections/reversals and snapshots are source-record
//! semantics and must not be guessed by the accounting aggregator.

pub mod event_semantics;

use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Stage {
    Planned,
    Simulated,
    Observed,
}

impl Stage {
    const fn index(self) -> usize {
        match self {
            Self::Planned => 0,
            Self::Simulated => 1,
            Self::Observed => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidencePermission {
    PublicOrPermitted,
    RestrictedButAuthorized,
    Unresolved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuantityRate {
    /// Estimated minor currency units per quantity unit. Fractional minor units are allowed
    /// here because these are model estimates, not posted financial transactions.
    pub unit_rate: f64,
    /// Must identify currency, minor-unit denomination, price year and region/basis.
    pub currency_basis_id: String,
    /// Number of decimal digits in one major currency unit (e.g. 2 for ZAR cents).
    pub minor_unit_exponent: u8,
    /// Immutable identity for the rate-card/method revision used.
    pub rate_card_ref: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuantityAggregationSemantics {
    /// The value is a contribution that may be added to other contributions.
    AdditiveContribution,
    /// The value is a cumulative state reading and must be normalized before audit.
    CumulativeSnapshot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuantityEvent {
    /// Semantic identity such as labor.welder.hours or material.aluminium.kg.
    pub quantity_key: String,
    /// Numeric value. The aggregation semantics below determine whether audit may sum it.
    pub quantity: f64,
    pub aggregation_semantics: QuantityAggregationSemantics,
    pub unit: String,
    /// Optional estimate from a declared and versioned rate card.
    pub rate: Option<QuantityRate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinancialTransaction {
    /// Stable line-level transaction allocation key, for example invoice-123:line-04.
    /// Unique per build; do not use only an invoice header when it has multiple lines.
    pub transaction_ref: String,
    /// Exact signed minor units; negative values represent explicit credit notes.
    /// The currency_basis_id must define the minor-unit scale (for example, ZAR cents).
    pub amount_minor_units: i64,
    pub currency_basis_id: String,
    /// Must agree with this basis ID everywhere it appears in the ledger.
    pub minor_unit_exponent: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventKind {
    Quantity(QuantityEvent),
    /// Direct financial actual. V0 admits these only at Observed stage.
    FinancialActual(FinancialTransaction),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LedgerEvent {
    pub event_id: String,
    pub build_id: String,
    /// SHA-256 of the exact vessel configuration/revision used for this event.
    pub configuration_sha256: String,
    pub task_id: String,
    pub stage: Stage,
    /// Stable source record, measurement, simulation receipt, work log, or transaction source.
    pub source_ref: String,
    pub source_permission: EvidencePermission,
    /// Exact reference to public licence terms or a reviewed authorization record.
    pub source_permission_ref: String,
    /// Explicit lineage links; references must resolve to events in the same ledger.
    pub related_event_ids: Vec<String>,
    pub kind: EventKind,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct QuantityGroupKey {
    build_id: String,
    configuration_sha256: String,
    task_id: String,
    quantity_key: String,
    unit: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct RateCardGroupKey {
    build_id: String,
    configuration_sha256: String,
    task_id: String,
    quantity_key: String,
    unit: String,
    stage: Stage,
    currency_basis_id: String,
    minor_unit_exponent: u8,
    rate_card_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct CurrencyGroupKey {
    build_id: String,
    configuration_sha256: String,
    task_id: String,
    currency_basis_id: String,
    minor_unit_exponent: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantityDisposition {
    Comparable,
    MissingPlan,
    MissingObserved,
    MissingPlanAndObserved,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuantityReconciliation {
    pub build_id: String,
    pub configuration_sha256: String,
    pub task_id: String,
    pub quantity_key: String,
    pub unit: String,
    /// None means no event was provided at this stage; it does not mean zero.
    pub planned: Option<f64>,
    pub simulated: Option<f64>,
    pub observed: Option<f64>,
    /// Observed minus planned; available only when both quantities are evidenced.
    pub observed_minus_planned: Option<f64>,
    pub disposition: QuantityDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RateCardCostTotal {
    pub build_id: String,
    pub configuration_sha256: String,
    pub task_id: String,
    pub quantity_key: String,
    pub unit: String,
    pub stage: Stage,
    pub currency_basis_id: String,
    pub minor_unit_exponent: u8,
    pub rate_card_ref: String,
    /// Quantity × rate estimate, never an invoice actual.
    pub amount: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinancialActualTotal {
    pub build_id: String,
    pub configuration_sha256: String,
    pub task_id: String,
    pub currency_basis_id: String,
    pub minor_unit_exponent: u8,
    /// Exact sum of observed direct financial transactions in minor currency units.
    pub amount_minor_units: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AuditReport {
    pub event_count: usize,
    pub quantity_reconciliations: Vec<QuantityReconciliation>,
    pub rate_card_costs: Vec<RateCardCostTotal>,
    pub financial_actuals: Vec<FinancialActualTotal>,
}

fn is_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn add_finite(current: f64, addition: f64) -> Option<f64> {
    let result = current + addition;
    result.is_finite().then_some(result)
}

fn validate_event(event: &LedgerEvent, errors: &mut Vec<String>) {
    let label = if is_nonempty(&event.event_id) {
        event.event_id.as_str()
    } else {
        "<missing-event-id>"
    };

    for (name, value) in [
        ("event_id", event.event_id.as_str()),
        ("build_id", event.build_id.as_str()),
        ("task_id", event.task_id.as_str()),
        ("source_ref", event.source_ref.as_str()),
        (
            "source_permission_ref",
            event.source_permission_ref.as_str(),
        ),
    ] {
        if !is_nonempty(value) {
            errors.push(format!("{label}: {name} is missing"));
        }
    }
    if !is_sha256(&event.configuration_sha256) {
        errors.push(format!(
            "{label}: configuration_sha256 must be 64 hexadecimal characters"
        ));
    }
    if event.source_permission == EvidencePermission::Unresolved {
        errors.push(format!(
            "{label}: source rights/access disposition is unresolved"
        ));
    }

    let mut related = HashSet::new();
    for related_id in &event.related_event_ids {
        if !is_nonempty(related_id) {
            errors.push(format!("{label}: related_event_ids contains a blank id"));
        }
        if related_id == &event.event_id {
            errors.push(format!(
                "{label}: event cannot reference itself as a related event"
            ));
        }
        if !related.insert(related_id.as_str()) {
            errors.push(format!(
                "{label}: duplicate related event reference {related_id}"
            ));
        }
    }

    match &event.kind {
        EventKind::Quantity(quantity) => {
            if quantity.aggregation_semantics != QuantityAggregationSemantics::AdditiveContribution
            {
                errors.push(format!(
                    "{label}: cumulative snapshots must be normalized before audit; only additive contributions are accepted"
                ));
            }
            if !is_nonempty(&quantity.quantity_key) {
                errors.push(format!("{label}: quantity_key is missing"));
            }
            if !is_nonempty(&quantity.unit) {
                errors.push(format!("{label}: quantity unit is missing"));
            }
            if !quantity.quantity.is_finite() || quantity.quantity < 0.0 {
                errors.push(format!("{label}: quantity must be finite and non-negative"));
            }
            if let Some(rate) = &quantity.rate {
                if !rate.unit_rate.is_finite() || rate.unit_rate < 0.0 {
                    errors.push(format!(
                        "{label}: unit_rate must be finite and non-negative"
                    ));
                }
                if !is_nonempty(&rate.currency_basis_id) {
                    errors.push(format!("{label}: rated quantity has no currency_basis_id"));
                }
                if rate.minor_unit_exponent > 6 {
                    errors.push(format!("{label}: minor_unit_exponent must be within 0..=6"));
                }
                if !is_nonempty(&rate.rate_card_ref) {
                    errors.push(format!("{label}: rated quantity has no rate_card_ref"));
                }
            }
        }
        EventKind::FinancialActual(transaction) => {
            if event.stage != Stage::Observed {
                errors.push(format!(
                    "{label}: financial actuals must have Observed stage"
                ));
            }
            if !is_nonempty(&transaction.transaction_ref) {
                errors.push(format!(
                    "{label}: financial actual requires transaction_ref"
                ));
            }
            if !is_nonempty(&transaction.currency_basis_id) {
                errors.push(format!(
                    "{label}: financial actual has no currency_basis_id"
                ));
            }
            if transaction.minor_unit_exponent > 6 {
                errors.push(format!("{label}: minor_unit_exponent must be within 0..=6"));
            }
        }
    }
}

fn find_lineage_cycles<'a>(
    events: &'a [LedgerEvent],
    events_by_id: &BTreeMap<String, &'a LedgerEvent>,
) -> Vec<String> {
    // Iterative DFS avoids call-stack growth for large ledgers.
    // State values: 0 = unseen, 1 = active, 2 = complete.
    let mut states: BTreeMap<String, u8> = BTreeMap::new();
    let mut errors = Vec::new();

    for event in events {
        let root = event.event_id.as_str();
        if states.get(root) == Some(&2) {
            continue;
        }

        let mut stack: Vec<(&str, bool)> = vec![(root, false)];
        while let Some((event_id, exiting)) = stack.pop() {
            if exiting {
                states.insert(event_id.to_owned(), 2);
                continue;
            }

            match states.get(event_id).copied().unwrap_or(0) {
                2 => continue,
                1 => {
                    errors.push(format!("{event_id}: cyclic related-event lineage"));
                    continue;
                }
                _ => {}
            }

            states.insert(event_id.to_owned(), 1);
            stack.push((event_id, true));
            if let Some(current) = events_by_id.get(event_id) {
                for related_id in current.related_event_ids.iter().rev() {
                    let parent = related_id.as_str();
                    if !events_by_id.contains_key(parent) {
                        continue;
                    }
                    match states.get(parent).copied().unwrap_or(0) {
                        1 => errors.push(format!("{parent}: cyclic related-event lineage")),
                        2 => {}
                        _ => stack.push((parent, false)),
                    }
                }
            }
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

/// Reconcile a ledger without converting absent evidence to zero.
///
/// Output families are deliberately non-additive:
/// - quantity variance only where plan and observed quantities exist for the same build,
///   configuration, task, semantic axis, and unit;
/// - rate-card estimates grouped by build/configuration/stage/money basis/rate revision;
/// - direct observed financial actuals grouped by build/configuration and money basis.
///
/// An observed quantity costed with a rate card and an invoice for that work are separate
/// analytical views until an explicit, source-backed allocation reconciles them.
pub fn audit(events: &[LedgerEvent]) -> Result<AuditReport, Vec<String>> {
    if events.is_empty() {
        return Err(vec![
            "ledger contains no events; empty evidence is not a zero-cost build".to_owned(),
        ]);
    }

    let mut errors = Vec::new();
    let mut event_ids: HashSet<&str> = HashSet::new();
    let mut transaction_refs: HashSet<(&str, &str)> = HashSet::new();

    for event in events {
        if is_nonempty(&event.event_id) && !event_ids.insert(event.event_id.as_str()) {
            errors.push(format!("{}: duplicate event_id", event.event_id));
        }
        if let EventKind::FinancialActual(transaction) = &event.kind
            && is_nonempty(&transaction.transaction_ref)
        {
            let key = (
                event.build_id.as_str(),
                transaction.transaction_ref.as_str(),
            );
            if !transaction_refs.insert(key) {
                errors.push(format!(
                    "{}: duplicate financial transaction allocation {}",
                    event.event_id, transaction.transaction_ref
                ));
            }
        }
        validate_event(event, &mut errors);
    }

    let events_by_id: BTreeMap<String, &LedgerEvent> = events
        .iter()
        .map(|event| (event.event_id.clone(), event))
        .collect();

    for event in events {
        for related_id in &event.related_event_ids {
            match events_by_id.get(related_id.as_str()) {
                None => errors.push(format!(
                    "{}: related event {} does not exist in this ledger",
                    event.event_id, related_id
                )),
                Some(related) => {
                    if related.build_id != event.build_id {
                        errors.push(format!(
                            "{}: related event {} belongs to a different build",
                            event.event_id, related_id
                        ));
                    }
                    if related.configuration_sha256 != event.configuration_sha256 {
                        errors.push(format!(
                            "{}: related event {} belongs to a different configuration revision",
                            event.event_id, related_id
                        ));
                    }
                    if related.stage > event.stage {
                        errors.push(format!(
                            "{}: related event {} is from a later evidence stage",
                            event.event_id, related_id
                        ));
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        errors.extend(find_lineage_cycles(events, &events_by_id));
    }

    let mut currency_scales: BTreeMap<String, u8> = BTreeMap::new();
    for event in events {
        let basis_scale = match &event.kind {
            EventKind::Quantity(quantity) => quantity
                .rate
                .as_ref()
                .map(|rate| (rate.currency_basis_id.as_str(), rate.minor_unit_exponent)),
            EventKind::FinancialActual(transaction) => Some((
                transaction.currency_basis_id.as_str(),
                transaction.minor_unit_exponent,
            )),
        };
        if let Some((basis_id, exponent)) = basis_scale {
            if let Some(previous) = currency_scales.get(basis_id) {
                if *previous != exponent {
                    errors.push(format!(
                        "{}: currency_basis_id {basis_id} has conflicting minor_unit_exponent values",
                        event.event_id
                    ));
                }
            } else {
                currency_scales.insert(basis_id.to_owned(), exponent);
            }
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut quantities: BTreeMap<QuantityGroupKey, [Option<f64>; 3]> = BTreeMap::new();
    let mut estimated_costs: BTreeMap<RateCardGroupKey, f64> = BTreeMap::new();
    let mut financials: BTreeMap<CurrencyGroupKey, i64> = BTreeMap::new();

    // Stable accumulation order is required; floating-point addition is not associative.
    let mut ordered_events: Vec<&LedgerEvent> = events.iter().collect();
    ordered_events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    for event in ordered_events {
        match &event.kind {
            EventKind::Quantity(quantity) => {
                let key = QuantityGroupKey {
                    build_id: event.build_id.clone(),
                    configuration_sha256: event.configuration_sha256.clone(),
                    task_id: event.task_id.clone(),
                    quantity_key: quantity.quantity_key.clone(),
                    unit: quantity.unit.clone(),
                };
                let buckets = quantities.entry(key).or_insert([None, None, None]);
                let index = event.stage.index();
                let sum = add_finite(buckets[index].unwrap_or(0.0), quantity.quantity).ok_or_else(
                    || vec![format!("{}: quantity aggregation overflow", event.event_id)],
                )?;
                buckets[index] = Some(sum);

                if let Some(rate) = &quantity.rate {
                    let estimated_amount = quantity.quantity * rate.unit_rate;
                    if !estimated_amount.is_finite() {
                        return Err(vec![format!(
                            "{}: quantity × rate overflow",
                            event.event_id
                        )]);
                    }
                    let cost_key = RateCardGroupKey {
                        build_id: event.build_id.clone(),
                        configuration_sha256: event.configuration_sha256.clone(),
                        task_id: event.task_id.clone(),
                        quantity_key: quantity.quantity_key.clone(),
                        unit: quantity.unit.clone(),
                        stage: event.stage,
                        currency_basis_id: rate.currency_basis_id.clone(),
                        minor_unit_exponent: rate.minor_unit_exponent,
                        rate_card_ref: rate.rate_card_ref.clone(),
                    };
                    let prior = estimated_costs.get(&cost_key).copied().unwrap_or(0.0);
                    let sum = add_finite(prior, estimated_amount).ok_or_else(|| {
                        vec![format!(
                            "{}: rate-card cost aggregation overflow",
                            event.event_id
                        )]
                    })?;
                    estimated_costs.insert(cost_key, sum);
                }
            }
            EventKind::FinancialActual(transaction) => {
                let key = CurrencyGroupKey {
                    build_id: event.build_id.clone(),
                    configuration_sha256: event.configuration_sha256.clone(),
                    task_id: event.task_id.clone(),
                    currency_basis_id: transaction.currency_basis_id.clone(),
                    minor_unit_exponent: transaction.minor_unit_exponent,
                };
                let prior = financials.get(&key).copied().unwrap_or(0_i64);
                let sum = prior
                    .checked_add(transaction.amount_minor_units)
                    .ok_or_else(|| {
                        vec![format!(
                            "{}: financial actual integer minor-unit overflow",
                            event.event_id
                        )]
                    })?;
                financials.insert(key, sum);
            }
        }
    }

    let quantity_reconciliations = quantities
        .into_iter()
        .map(|(key, values)| {
            let planned = values[Stage::Planned.index()];
            let simulated = values[Stage::Simulated.index()];
            let observed = values[Stage::Observed.index()];
            let observed_minus_planned = match (observed, planned) {
                (Some(actual), Some(plan)) => Some(actual - plan),
                _ => None,
            };
            let disposition = match (planned, observed) {
                (Some(_), Some(_)) => QuantityDisposition::Comparable,
                (Some(_), None) => QuantityDisposition::MissingObserved,
                (None, Some(_)) => QuantityDisposition::MissingPlan,
                (None, None) => QuantityDisposition::MissingPlanAndObserved,
            };
            QuantityReconciliation {
                build_id: key.build_id,
                configuration_sha256: key.configuration_sha256,
                task_id: key.task_id,
                quantity_key: key.quantity_key,
                unit: key.unit,
                planned,
                simulated,
                observed,
                observed_minus_planned,
                disposition,
            }
        })
        .collect();

    let rate_card_costs = estimated_costs
        .into_iter()
        .map(|(key, amount)| RateCardCostTotal {
            build_id: key.build_id,
            configuration_sha256: key.configuration_sha256,
            task_id: key.task_id,
            quantity_key: key.quantity_key,
            unit: key.unit,
            stage: key.stage,
            currency_basis_id: key.currency_basis_id,
            minor_unit_exponent: key.minor_unit_exponent,
            rate_card_ref: key.rate_card_ref,
            amount,
        })
        .collect();

    let financial_actuals = financials
        .into_iter()
        .map(|(key, amount)| FinancialActualTotal {
            build_id: key.build_id,
            configuration_sha256: key.configuration_sha256,
            task_id: key.task_id,
            currency_basis_id: key.currency_basis_id,
            minor_unit_exponent: key.minor_unit_exponent,
            amount_minor_units: amount,
        })
        .collect();

    Ok(AuditReport {
        event_count: events.len(),
        quantity_reconciliations,
        rate_card_costs,
        financial_actuals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CONFIG_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn quantity_event(
        event_id: &str,
        stage: Stage,
        config: &str,
        task: &str,
        key: &str,
        quantity: f64,
        rate: Option<QuantityRate>,
    ) -> LedgerEvent {
        LedgerEvent {
            event_id: event_id.to_owned(),
            build_id: "build-001".to_owned(),
            configuration_sha256: config.to_owned(),
            task_id: task.to_owned(),
            stage,
            source_ref: format!("evidence:{event_id}"),
            source_permission: EvidencePermission::PublicOrPermitted,
            source_permission_ref: "fixture-rights:public-test-data".to_owned(),
            related_event_ids: Vec::new(),
            kind: EventKind::Quantity(QuantityEvent {
                quantity_key: key.to_owned(),
                quantity,
                aggregation_semantics: QuantityAggregationSemantics::AdditiveContribution,
                unit: "hour".to_owned(),
                rate,
            }),
        }
    }

    fn financial_event(event_id: &str, reference: &str, amount_minor_units: i64) -> LedgerEvent {
        LedgerEvent {
            event_id: event_id.to_owned(),
            build_id: "build-001".to_owned(),
            configuration_sha256: CONFIG_A.to_owned(),
            task_id: "weld-01".to_owned(),
            stage: Stage::Observed,
            source_ref: format!("invoice:{reference}"),
            source_permission: EvidencePermission::PublicOrPermitted,
            source_permission_ref: "fixture-rights:public-test-data".to_owned(),
            related_event_ids: Vec::new(),
            kind: EventKind::FinancialActual(FinancialTransaction {
                transaction_ref: reference.to_owned(),
                amount_minor_units,
                currency_basis_id: "ZAR-cent-2026-SA".to_owned(),
                minor_unit_exponent: 2,
            }),
        }
    }

    fn rate() -> QuantityRate {
        QuantityRate {
            unit_rate: 10_000.0,
            currency_basis_id: "ZAR-cent-2026-SA".to_owned(),
            minor_unit_exponent: 2,
            rate_card_ref: "rate-card-v1".to_owned(),
        }
    }

    #[test]
    fn computes_variance_only_when_plan_and_observation_exist() {
        let events = vec![
            quantity_event(
                "plan",
                Stage::Planned,
                CONFIG_A,
                "weld-01",
                "labor.welder",
                10.0,
                Some(rate()),
            ),
            quantity_event(
                "actual",
                Stage::Observed,
                CONFIG_A,
                "weld-01",
                "labor.welder",
                12.0,
                Some(rate()),
            ),
        ];
        let report = audit(&events).unwrap();
        assert_eq!(report.quantity_reconciliations.len(), 1);
        let row = &report.quantity_reconciliations[0];
        assert_eq!(row.planned, Some(10.0));
        assert_eq!(row.observed, Some(12.0));
        assert_eq!(row.observed_minus_planned, Some(2.0));
        assert_eq!(row.disposition, QuantityDisposition::Comparable);
        assert_eq!(report.rate_card_costs.len(), 2);
    }

    #[test]
    fn simulated_quantities_do_not_become_observed_actuals() {
        let events = vec![
            quantity_event(
                "plan",
                Stage::Planned,
                CONFIG_A,
                "cut-01",
                "machine.laser",
                3.0,
                None,
            ),
            quantity_event(
                "sim",
                Stage::Simulated,
                CONFIG_A,
                "cut-01",
                "machine.laser",
                2.0,
                None,
            ),
        ];
        let report = audit(&events).unwrap();
        let row = &report.quantity_reconciliations[0];
        assert_eq!(row.planned, Some(3.0));
        assert_eq!(row.simulated, Some(2.0));
        assert_eq!(row.observed, None);
        assert_eq!(row.observed_minus_planned, None);
        assert_eq!(row.disposition, QuantityDisposition::MissingObserved);
    }

    #[test]
    fn absent_planned_quantity_is_not_silently_zero() {
        let events = vec![quantity_event(
            "actual",
            Stage::Observed,
            CONFIG_A,
            "paint-01",
            "labor.painter",
            4.0,
            None,
        )];
        let report = audit(&events).unwrap();
        let row = &report.quantity_reconciliations[0];
        assert_eq!(row.planned, None);
        assert_eq!(row.observed, Some(4.0));
        assert_eq!(row.observed_minus_planned, None);
        assert_eq!(row.disposition, QuantityDisposition::MissingPlan);
    }

    #[test]
    fn plan_and_actual_on_different_configuration_revisions_do_not_compare() {
        let events = vec![
            quantity_event(
                "plan",
                Stage::Planned,
                CONFIG_A,
                "weld-01",
                "labor.welder",
                10.0,
                None,
            ),
            quantity_event(
                "actual",
                Stage::Observed,
                CONFIG_B,
                "weld-01",
                "labor.welder",
                12.0,
                None,
            ),
        ];
        let report = audit(&events).unwrap();
        assert_eq!(report.quantity_reconciliations.len(), 2);
        assert!(
            report
                .quantity_reconciliations
                .iter()
                .all(|row| row.observed_minus_planned.is_none())
        );
    }

    #[test]
    fn financial_actuals_are_not_added_to_rate_card_costs() {
        let events = vec![
            quantity_event(
                "actual-hours",
                Stage::Observed,
                CONFIG_A,
                "weld-01",
                "labor.welder",
                2.0,
                Some(rate()),
            ),
            financial_event("invoice-line", "inv-42:line-3", 35_000),
        ];
        let report = audit(&events).unwrap();
        assert_eq!(
            report
                .rate_card_costs
                .iter()
                .find(|row| row.stage == Stage::Observed)
                .unwrap()
                .amount,
            20_000.0
        );
        assert_eq!(report.financial_actuals.len(), 1);
        assert_eq!(report.financial_actuals[0].amount_minor_units, 35_000);
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let events = vec![
            quantity_event(
                "same",
                Stage::Planned,
                CONFIG_A,
                "weld-01",
                "labor",
                1.0,
                None,
            ),
            quantity_event(
                "same",
                Stage::Observed,
                CONFIG_A,
                "weld-01",
                "labor",
                2.0,
                None,
            ),
        ];
        let errors = audit(&events).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate event_id"))
        );
    }

    #[test]
    fn duplicate_financial_allocation_is_rejected() {
        let events = vec![
            financial_event("event-1", "inv-42:line-3", 35_000),
            financial_event("event-2", "inv-42:line-3", 35_000),
        ];
        let errors = audit(&events).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate financial transaction allocation"))
        );
    }

    #[test]
    fn financial_actual_in_planned_stage_is_rejected() {
        let mut event = financial_event("event-1", "inv-42:line-3", 35_000);
        event.stage = Stage::Planned;
        let errors = audit(&[event]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("must have Observed stage"))
        );
    }

    #[test]
    fn missing_source_reference_is_rejected() {
        let mut event = quantity_event(
            "event-1",
            Stage::Observed,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        event.source_ref = " ".to_owned();
        let errors = audit(&[event]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("source_ref is missing"))
        );
    }

    #[test]
    fn source_permission_needs_a_reference_not_only_an_enum() {
        let mut event = quantity_event(
            "event-1",
            Stage::Observed,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        event.source_permission_ref.clear();
        let errors = audit(&[event]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("source_permission_ref is missing"))
        );
    }

    #[test]
    fn unresolved_source_rights_are_rejected() {
        let mut event = quantity_event(
            "event-1",
            Stage::Observed,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        event.source_permission = EvidencePermission::Unresolved;
        let errors = audit(&[event]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("rights/access disposition is unresolved"))
        );
    }

    #[test]
    fn malformed_configuration_digest_is_rejected() {
        let event = quantity_event(
            "event-1",
            Stage::Observed,
            "not-a-digest",
            "weld-01",
            "labor",
            1.0,
            None,
        );
        let errors = audit(&[event]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("configuration_sha256"))
        );
    }

    #[test]
    fn rate_without_basis_or_revision_is_rejected() {
        let mut invalid_rate = rate();
        invalid_rate.currency_basis_id.clear();
        invalid_rate.rate_card_ref.clear();
        let event = quantity_event(
            "event-1",
            Stage::Planned,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            Some(invalid_rate),
        );
        let errors = audit(&[event]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("no currency_basis_id"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("no rate_card_ref"))
        );
    }

    #[test]
    fn derived_cost_overflow_is_rejected() {
        let mut enormous_rate = rate();
        enormous_rate.unit_rate = f64::MAX;
        let event = quantity_event(
            "event-1",
            Stage::Observed,
            CONFIG_A,
            "weld-01",
            "labor",
            2.0,
            Some(enormous_rate),
        );
        let errors = audit(&[event]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("quantity × rate overflow"))
        );
    }

    #[test]
    fn unknown_lineage_reference_is_rejected() {
        let mut event = quantity_event(
            "event-1",
            Stage::Observed,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        event.related_event_ids.push("missing-parent".to_owned());
        let errors = audit(&[event]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("does not exist in this ledger"))
        );
    }

    #[test]
    fn related_event_from_another_build_is_rejected() {
        let mut event = quantity_event(
            "observed",
            Stage::Observed,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        event.related_event_ids.push("plan".to_owned());
        let mut plan = quantity_event(
            "plan",
            Stage::Planned,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        plan.build_id = "different-build".to_owned();
        let errors = audit(&[event, plan]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("belongs to a different build"))
        );
    }

    #[test]
    fn related_event_from_different_configuration_is_rejected() {
        let mut event = quantity_event(
            "observed",
            Stage::Observed,
            CONFIG_B,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        event.related_event_ids.push("plan".to_owned());
        let plan = quantity_event(
            "plan",
            Stage::Planned,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        let errors = audit(&[event, plan]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("different configuration revision"))
        );
    }

    #[test]
    fn later_stage_cannot_be_a_parent_of_an_earlier_stage() {
        let mut event = quantity_event(
            "plan",
            Stage::Planned,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        event.related_event_ids.push("actual".to_owned());
        let actual = quantity_event(
            "actual",
            Stage::Observed,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        let errors = audit(&[event, actual]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("later evidence stage"))
        );
    }

    #[test]
    fn lineage_cycle_is_rejected() {
        let mut first = quantity_event(
            "first",
            Stage::Observed,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            None,
        );
        let mut second = quantity_event(
            "second",
            Stage::Observed,
            CONFIG_A,
            "weld-01",
            "labor",
            2.0,
            None,
        );
        first.related_event_ids.push("second".to_owned());
        second.related_event_ids.push("first".to_owned());
        let errors = audit(&[first, second]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("cyclic related-event lineage"))
        );
    }

    #[test]
    fn financial_actuals_for_distinct_configuration_revisions_are_not_merged() {
        let mut revised = financial_event("invoice-revised", "inv-43:line-1", 12_500);
        revised.configuration_sha256 = CONFIG_B.to_owned();
        let original = financial_event("invoice-original", "inv-42:line-1", 20_000);
        let report = audit(&[original, revised]).unwrap();
        assert_eq!(report.financial_actuals.len(), 2);
        assert!(report.financial_actuals.iter().any(|row| row.configuration_sha256 == CONFIG_A && row.amount_minor_units == 20_000));
        assert!(report.financial_actuals.iter().any(|row| row.configuration_sha256 == CONFIG_B && row.amount_minor_units == 12_500));
    }

    #[test]
    fn large_lineage_chain_is_audited_iteratively() {
        let mut events = Vec::new();
        for index in (0..5_000).rev() {
            let event_id = format!("event-{index}");
            let mut event = quantity_event(
                &event_id,
                Stage::Observed,
                CONFIG_A,
                "fabrication-1",
                "labor.fitter.hours",
                1.0,
                None,
            );
            if index > 0 {
                event.related_event_ids.push(format!("event-{}", index - 1));
            }
            events.push(event);
        }

        let report = audit(&events).expect("large acyclic lineage");
        assert_eq!(report.event_count, 5_000);
        assert_eq!(report.quantity_reconciliations.len(), 1);
        assert_eq!(report.quantity_reconciliations[0].observed, Some(5_000.0));
    }

    #[test]
    fn empty_ledger_is_not_reported_as_zero_cost() {
        let errors = audit(&[]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("empty evidence is not a zero-cost build"))
        );
    }

    #[test]
    fn same_stage_float_aggregation_is_independent_of_input_order() {
        let first = vec![
            quantity_event(
                "a-large",
                Stage::Observed,
                CONFIG_A,
                "weld-01",
                "labor",
                1.0e16,
                None,
            ),
            quantity_event(
                "b-small",
                Stage::Observed,
                CONFIG_A,
                "weld-01",
                "labor",
                1.0,
                None,
            ),
            quantity_event(
                "c-small",
                Stage::Observed,
                CONFIG_A,
                "weld-01",
                "labor",
                1.0,
                None,
            ),
        ];
        let mut second = first.clone();
        second.reverse();
        assert_eq!(audit(&first).unwrap(), audit(&second).unwrap());
    }

    #[test]
    fn input_permutation_does_not_change_report() {
        let first = vec![
            quantity_event(
                "plan",
                Stage::Planned,
                CONFIG_A,
                "weld-01",
                "labor.welder",
                10.0,
                Some(rate()),
            ),
            quantity_event(
                "actual",
                Stage::Observed,
                CONFIG_A,
                "weld-01",
                "labor.welder",
                12.0,
                Some(rate()),
            ),
            financial_event("invoice-line", "inv-42:line-3", 35_000),
        ];
        let mut second = first.clone();
        second.reverse();
        assert_eq!(audit(&first).unwrap(), audit(&second).unwrap());
    }

    #[test]
    fn rate_card_estimates_remain_separated_by_work_package() {
        let welding = quantity_event(
            "weld-hours",
            Stage::Observed,
            CONFIG_A,
            "welding-block-a",
            "labor.welder.hours",
            1.0,
            Some(rate()),
        );
        let painting = quantity_event(
            "paint-hours",
            Stage::Observed,
            CONFIG_A,
            "painting-block-a",
            "labor.painter.hours",
            2.0,
            Some(rate()),
        );
        let report = audit(&[welding, painting]).unwrap();
        assert_eq!(report.rate_card_costs.len(), 2);
        assert!(
            report
                .rate_card_costs
                .iter()
                .any(|row| row.task_id == "welding-block-a" && row.amount == 10_000.0)
        );
        assert!(
            report
                .rate_card_costs
                .iter()
                .any(|row| row.task_id == "painting-block-a" && row.amount == 20_000.0)
        );
    }

    #[test]
    fn financial_actuals_remain_separated_by_work_package() {
        let mut welding = financial_event("invoice-weld", "inv-1:line-1", 30_000);
        welding.task_id = "welding-block-a".to_owned();
        let mut painting = financial_event("invoice-paint", "inv-2:line-1", 12_000);
        painting.task_id = "paint-block-a".to_owned();
        let report = audit(&[welding, painting]).unwrap();
        assert_eq!(report.financial_actuals.len(), 2);
        assert!(
            report
                .financial_actuals
                .iter()
                .any(|row| row.task_id == "welding-block-a" && row.amount_minor_units == 30_000)
        );
        assert!(
            report
                .financial_actuals
                .iter()
                .any(|row| row.task_id == "paint-block-a" && row.amount_minor_units == 12_000)
        );
    }

    #[test]
    fn financial_actuals_sum_exact_minor_units_including_credit_notes() {
        let events = vec![
            financial_event("invoice", "inv-1:line-1", 35_000),
            financial_event("credit", "credit-1:line-1", -5_001),
        ];
        let report = audit(&events).unwrap();
        assert_eq!(report.financial_actuals.len(), 1);
        assert_eq!(report.financial_actuals[0].amount_minor_units, 29_999);
    }

    #[test]
    fn currency_basis_rejects_conflicting_minor_unit_exponents() {
        let planned = quantity_event(
            "rate",
            Stage::Planned,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            Some(rate()),
        );
        let mut actual = financial_event("invoice", "inv-1:line-1", 10_000);
        if let EventKind::FinancialActual(transaction) = &mut actual.kind {
            transaction.minor_unit_exponent = 0;
        }
        let errors = audit(&[planned, actual]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("conflicting minor_unit_exponent values"))
        );
    }

    #[test]
    fn audit_rejects_cumulative_snapshot_semantics_before_aggregation() {
        let mut event = quantity_event(
            "snapshot",
            Stage::Observed,
            CONFIG_A,
            "weld-01",
            "labor",
            12.0,
            None,
        );
        if let EventKind::Quantity(quantity) = &mut event.kind {
            quantity.aggregation_semantics = QuantityAggregationSemantics::CumulativeSnapshot;
        }
        let errors = audit(&[event]).unwrap_err();
        assert!(errors.iter().any(|error| error.contains(
            "cumulative snapshots must be normalized before audit"
        )));
    }

    #[test]
    fn minor_unit_exponents_outside_supported_range_are_rejected() {
        let mut invalid_rate = rate();
        invalid_rate.minor_unit_exponent = 7;
        let event = quantity_event(
            "rate",
            Stage::Planned,
            CONFIG_A,
            "weld-01",
            "labor",
            1.0,
            Some(invalid_rate),
        );
        let errors = audit(&[event]).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("minor_unit_exponent must be within 0..=6"))
        );
    }

    #[test]
    fn financial_minor_unit_overflow_is_rejected() {
        let events = vec![
            financial_event("event-1", "inv-42:line-3", i64::MAX),
            financial_event("event-2", "inv-43:line-3", 1),
        ];
        let errors = audit(&events).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("financial actual integer minor-unit overflow"))
        );
    }
}

//! Exact E0 reference oracle for protective-resilience portfolio evidence.
//!
//! The input contains already-declared synthetic service trajectories. This
//! binary independently recomputes service-deficit and sustained-recovery
//! metrics, checks matched scope and cost/benefit identity, and enforces
//! non-compensatory safety decisions. It is not a hazard or service simulator.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::process;

const FIXTURE: &str =
    include_str!("../../../fixtures/protective-resilience-e0-reference-v0.1.json");
const REGISTRY: &str =
    include_str!("../../../fixtures/protective-threat-family-register-v0.1.json");
const SCHEMA: &str = "protective-resilience-e0/0.1";

#[derive(Clone, Debug, Default)]
struct ServiceAggregate {
    deficit_milli_beneficiary_minutes: u64,
}

#[derive(Clone, Debug)]
struct OutcomeMetric {
    outcome_id: String,
    service_id: String,
    deficit_milli_beneficiary_minutes: u64,
    minimum_delivery_permille: u64,
    intervals_below_floor: usize,
    sustained_recovery_start_minutes: Option<u64>,
}

fn required_text<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| format!("{path}.{key} must be non-empty text"))
}

fn required_u64(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{path}.{key} must be a non-negative integer"))
}

fn reject_unsupported_numeric_risk_claims(value: &Value, path: &str) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                let child_path = format!("{path}.{key}");
                let name = key.to_ascii_lowercase();
                let forbidden = [
                    "probability",
                    "event_probability",
                    "annual_probability",
                    "annual_frequency",
                    "expected_annual_loss",
                    "expected_loss",
                    "damage_magnitude",
                ]
                .contains(&name.as_str());
                if forbidden && child.is_number() {
                    return Err(format!(
                        "{child_path} is numeric without an admitted probability/loss evidence contract"
                    ));
                }
                reject_unsupported_numeric_risk_claims(child, &child_path)?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_unsupported_numeric_risk_claims(child, &format!("{path}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn scope_digest(scope: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(scope)
        .map_err(|error| format!("scope canonicalization failed: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn outcome_contracts(
    outcomes: &[Value],
    horizon: usize,
) -> Result<BTreeMap<(String, String), (String, u64, u64)>, String> {
    if outcomes.is_empty() {
        return Err("service_outcomes must be a non-empty array".to_owned());
    }

    let mut contracts = BTreeMap::new();
    let mut outcome_ids = BTreeSet::new();

    for (index, outcome) in outcomes.iter().enumerate() {
        let path = format!("service_outcomes[{index}]");
        let outcome_id = required_text(outcome, "outcome_id", &path)?;
        if !outcome_ids.insert(outcome_id.to_owned()) {
            return Err(format!("{path}: duplicate outcome_id {outcome_id}"));
        }

        let service_id = required_text(outcome, "service_id", &path)?;
        let stratum = required_text(outcome, "beneficiary_stratum", &path)?;
        let unit = required_text(outcome, "beneficiary_unit", &path)?;
        let beneficiaries = required_u64(outcome, "beneficiary_count", &path)?;
        let floor = required_u64(outcome, "minimum_service_permille", &path)?;
        let delivered = outcome
            .get("delivered_service_permille")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{path}.delivered_service_permille must be an array"))?;

        if beneficiaries == 0 {
            return Err(format!("{path}.beneficiary_count must be positive"));
        }
        if !(1..=1000).contains(&floor) {
            return Err(format!("{path}.minimum_service_permille must be in 1..=1000"));
        }
        if delivered.len() != horizon {
            return Err(format!(
                "{path}: expected {horizon} trajectory intervals, found {}",
                delivered.len()
            ));
        }
        if delivered.iter().any(|item| {
            item.as_u64()
                .is_none_or(|ratio| ratio > 1000)
        }) {
            return Err(format!(
                "{path}.delivered_service_permille values must be integers in 0..=1000"
            ));
        }

        let key = (service_id.to_owned(), stratum.to_owned());
        if contracts
            .insert(key, (unit.to_owned(), beneficiaries, floor))
            .is_some()
        {
            return Err(format!(
                "{path}: duplicate service/beneficiary-stratum contract"
            ));
        }

        let origin = outcome
            .get("origin")
            .ok_or_else(|| format!("{path}.origin is required"))?;
        if required_text(origin, "kind", &format!("{path}.origin"))?
            != "SYNTHETIC_REFERENCE_TRAJECTORY"
        {
            return Err(format!("{path}.origin.kind must identify a synthetic reference"));
        }
        required_text(origin, "upstream_owner", &format!("{path}.origin"))?;
        required_text(origin, "source_receipt", &format!("{path}.origin"))?;
        required_text(origin, "claim_ceiling", &format!("{path}.origin"))?;
    }

    Ok(contracts)
}

fn evaluate_outcomes(
    outcomes: &[Value],
    horizon: usize,
    interval_minutes: u64,
    sustained_intervals: usize,
) -> Result<(Vec<OutcomeMetric>, BTreeMap<String, ServiceAggregate>), String> {
    let contracts = outcome_contracts(outcomes, horizon)?;
    let mut metrics = Vec::with_capacity(outcomes.len());
    let mut aggregates: BTreeMap<String, ServiceAggregate> = BTreeMap::new();

    for (index, outcome) in outcomes.iter().enumerate() {
        let path = format!("service_outcomes[{index}]");
        let outcome_id = required_text(outcome, "outcome_id", &path)?.to_owned();
        let service_id = required_text(outcome, "service_id", &path)?.to_owned();
        let stratum = required_text(outcome, "beneficiary_stratum", &path)?;
        let (unit, beneficiaries, floor) = contracts
            .get(&(service_id.clone(), stratum.to_owned()))
            .ok_or_else(|| format!("{path}: missing service/stratum contract"))?;
        if unit != "PERSON" {
            return Err(format!(
                "{path}: E0 v0.1 requires PERSON beneficiaries for this reference case"
            ));
        }

        let delivered: Vec<u64> = outcome["delivered_service_permille"]
            .as_array()
            .expect("validated above")
            .iter()
            .map(|item| item.as_u64().expect("validated above"))
            .collect();

        let mut deficit = 0u64;
        let mut below = 0usize;
        for ratio in &delivered {
            if ratio < floor {
                below += 1;
                let shortfall = floor - ratio;
                let interval_deficit = shortfall
                    .checked_mul(*beneficiaries)
                    .and_then(|value| value.checked_mul(interval_minutes))
                    .ok_or_else(|| format!("{path}: service-deficit arithmetic overflow"))?;
                deficit = deficit
                    .checked_add(interval_deficit)
                    .ok_or_else(|| format!("{path}: service-deficit accumulation overflow"))?;
            }
        }

        let minimum = delivered.iter().copied().min().unwrap_or(0);
        let recovery_index = delivered
            .windows(sustained_intervals)
            .position(|window| window.iter().all(|ratio| ratio >= floor));
        let recovery_minutes = recovery_index
            .map(|start| (start as u64).saturating_mul(interval_minutes));

        let aggregate = aggregates.entry(service_id.clone()).or_default();
        aggregate.deficit_milli_beneficiary_minutes = aggregate
            .deficit_milli_beneficiary_minutes
            .checked_add(deficit)
            .ok_or_else(|| format!("{path}: service aggregate overflow"))?;
        metrics.push(OutcomeMetric {
            outcome_id,
            service_id,
            deficit_milli_beneficiary_minutes: deficit,
            minimum_delivery_permille: minimum,
            intervals_below_floor: below,
            sustained_recovery_start_minutes: recovery_minutes,
        });
    }

    Ok((metrics, aggregates))
}

fn validate_benefit_claims(
    claims: &[Value],
    outcomes: &[Value],
    family_ids: &BTreeSet<String>,
) -> Result<(), String> {
    if claims.len() != outcomes.len() {
        return Err("each service outcome must have exactly one benefit claim".to_owned());
    }

    let outcome_ids: BTreeSet<String> = outcomes
        .iter()
        .map(|outcome| {
            required_text(outcome, "outcome_id", "service_outcome")
                .map(str::to_owned)
        })
        .collect::<Result<_, _>>()?;
    let mut claimed_outcomes = BTreeSet::new();
    let mut claim_ids = BTreeSet::new();

    for (index, claim) in claims.iter().enumerate() {
        let path = format!("benefit_claims[{index}]");
        let claim_id = required_text(claim, "claim_id", &path)?;
        if !claim_ids.insert(claim_id.to_owned()) {
            return Err(format!("{path}: duplicate claim_id {claim_id}"));
        }
        let outcome_id = required_text(claim, "outcome_id", &path)?;
        if !outcome_ids.contains(outcome_id) {
            return Err(format!("{path}: claim references unknown outcome {outcome_id}"));
        }
        if !claimed_outcomes.insert(outcome_id.to_owned()) {
            return Err(format!(
                "{path}: outcome {outcome_id} is credited more than once"
            ));
        }

        let related = claim
            .get("related_hazard_families")
            .and_then(Value::as_array)
            .filter(|items| !items.is_empty())
            .ok_or_else(|| format!("{path}.related_hazard_families must be non-empty"))?;
        let mut seen_families = BTreeSet::new();
        for family in related {
            let id = family
                .as_str()
                .ok_or_else(|| format!("{path}: hazard-family IDs must be text"))?;
            if !family_ids.contains(id) {
                return Err(format!("{path}: unknown hazard-family ID {id}"));
            }
            if !seen_families.insert(id.to_owned()) {
                return Err(format!("{path}: duplicate hazard-family link {id}"));
            }
        }
    }

    if claimed_outcomes != outcome_ids {
        return Err("benefit claims do not cover each service outcome exactly once".to_owned());
    }
    Ok(())
}

fn lifecycle_cost(portfolio: &Value) -> Result<u64, String> {
    let items = portfolio
        .get("cost_items")
        .and_then(Value::as_array)
        .ok_or_else(|| "cost_items must be an array".to_owned())?;
    let mut ids = BTreeSet::new();
    let mut total = 0u64;

    for (index, item) in items.iter().enumerate() {
        let path = format!("cost_items[{index}]");
        let id = required_text(item, "cost_id", &path)?;
        if !ids.insert(id.to_owned()) {
            return Err(format!("{path}: duplicate cost_id {id}"));
        }
        required_text(item, "capability_id", &path)?;
        required_text(item, "category", &path)?;
        let amount = required_u64(item, "present_value_minor_units", &path)?;
        total = total
            .checked_add(amount)
            .ok_or_else(|| "lifecycle cost overflow".to_owned())?;
    }
    Ok(total)
}

fn validate_status<'a>(
    value: &'a Value,
    label: &str,
    allowed: &[&str],
) -> Result<&'a str, String> {
    let status = required_text(value, "status", label)?;
    if !allowed.contains(&status) {
        return Err(format!("{label}.status is unsupported: {status}"));
    }
    required_text(value, "owner_contract", label)?;
    if status != "UNKNOWN" {
        required_text(value, "evidence_ref", label)?;
    }
    Ok(status)
}

fn service_totals_report(
    current: &BTreeMap<String, ServiceAggregate>,
    baseline: &BTreeMap<String, ServiceAggregate>,
    limits: &[Value],
    metric_by_service: &BTreeMap<String, Vec<&OutcomeMetric>>,
) -> Result<(Vec<Value>, bool), String> {
    let mut limit_map = BTreeMap::new();
    for (index, limit) in limits.iter().enumerate() {
        let path = format!("service_limits[{index}]");
        let service_id = required_text(limit, "service_id", &path)?;
        let max_deficit =
            required_u64(limit, "maximum_deficit_milli_beneficiary_minutes", &path)?;
        let max_recovery =
            required_u64(limit, "maximum_recovery_minutes_per_stratum", &path)?;
        if limit_map
            .insert(service_id.to_owned(), (max_deficit, max_recovery))
            .is_some()
        {
            return Err(format!("{path}: duplicate service limit for {service_id}"));
        }
    }

    if current.keys().ne(baseline.keys()) || limit_map.keys().ne(current.keys()) {
        return Err("ABSTAIN_INCOMPARABLE_SCOPE: service IDs or service-limit scope differ".to_owned());
    }

    let mut reports = Vec::with_capacity(current.len());
    let mut all_within = true;

    for (service_id, aggregate) in current {
        let (max_deficit, max_recovery) = limit_map
            .get(service_id)
            .ok_or_else(|| format!("no declared service limit for {service_id}"))?;
        let outcome_metrics = metric_by_service
            .get(service_id)
            .ok_or_else(|| format!("missing outcome metrics for {service_id}"))?;
        let all_recovered = outcome_metrics.iter().all(|metric| {
            metric
                .sustained_recovery_start_minutes
                .is_some_and(|minutes| minutes <= *max_recovery)
        });
        let within = aggregate.deficit_milli_beneficiary_minutes <= *max_deficit
            && all_recovered;
        all_within &= within;

        let baseline_deficit = baseline
            .get(service_id)
            .expect("matching keys checked")
            .deficit_milli_beneficiary_minutes;
        let reduction_basis_points = if baseline_deficit == 0 {
            None
        } else {
            let delta = i128::from(baseline_deficit)
                - i128::from(aggregate.deficit_milli_beneficiary_minutes);
            let basis_points = delta * 10_000 / i128::from(baseline_deficit);
            Some(
                i64::try_from(basis_points)
                    .map_err(|_| format!("{service_id}: improvement basis-points overflow"))?,
            )
        };

        reports.push(json!({
            "service_id": service_id,
            "deficit_milli_beneficiary_minutes": aggregate.deficit_milli_beneficiary_minutes,
            "baseline_deficit_milli_beneficiary_minutes": baseline_deficit,
            "deficit_reduction_basis_points": reduction_basis_points,
            "within_declared_limits": within,
            "all_strata_recovered": all_recovered
        }));
    }

    Ok((reports, all_within))
}

fn metrics_to_json(metrics: &[OutcomeMetric]) -> Vec<Value> {
    metrics
        .iter()
        .map(|metric| {
            json!({
                "outcome_id": metric.outcome_id,
                "deficit_milli_beneficiary_minutes": metric.deficit_milli_beneficiary_minutes,
                "minimum_delivery_permille": metric.minimum_delivery_permille,
                "intervals_below_floor": metric.intervals_below_floor,
                "sustained_recovery_start_minutes": metric.sustained_recovery_start_minutes
            })
        })
        .collect()
}

struct EvaluationContext<'a> {
    baseline_contracts: &'a BTreeMap<(String, String), (String, u64, u64)>,
    baseline_aggregates: &'a BTreeMap<String, ServiceAggregate>,
    scope: &'a Value,
    horizon: usize,
    interval_minutes: u64,
    sustained_intervals: usize,
    limits: &'a [Value],
    family_ids: &'a BTreeSet<String>,
}

fn evaluate_portfolio(
    portfolio: &Value,
    context: &EvaluationContext<'_>,
) -> Result<Value, String> {
    let portfolio_id = required_text(portfolio, "portfolio_id", "portfolio")?;
    if portfolio.get("scope") != Some(context.scope) {
        return Err(format!(
            "ABSTAIN_INCOMPARABLE_SCOPE: {portfolio_id} differs from the frozen baseline context"
        ));
    }

    let outcomes = portfolio
        .get("service_outcomes")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{portfolio_id}.service_outcomes must be an array"))?;
    let contracts = outcome_contracts(outcomes, context.horizon)?;
    if &contracts != context.baseline_contracts {
        return Err(format!(
            "ABSTAIN_INCOMPARABLE_SCOPE: {portfolio_id} changed service demand or beneficiary strata"
        ));
    }

    let (metrics, aggregates) =
        evaluate_outcomes(outcomes, context.horizon, context.interval_minutes, context.sustained_intervals)?;
    let mut metric_by_service: BTreeMap<String, Vec<&OutcomeMetric>> = BTreeMap::new();
    for metric in &metrics {
        metric_by_service
            .entry(metric.service_id.clone())
            .or_default()
            .push(metric);
    }

    let cost = lifecycle_cost(portfolio)?;
    let claims = portfolio
        .get("benefit_claims")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{portfolio_id}.benefit_claims must be an array"))?;
    validate_benefit_claims(claims, outcomes, context.family_ids)?;

    let safety = portfolio
        .get("mandatory_safety")
        .ok_or_else(|| format!("{portfolio_id}.mandatory_safety is required"))?;
    let safety_status = validate_status(
        safety,
        &format!("{portfolio_id}.mandatory_safety"),
        &["PASS", "VIOLATED", "UNKNOWN"],
    )?;
    let buffer = portfolio
        .get("buffer_recovery")
        .ok_or_else(|| format!("{portfolio_id}.buffer_recovery is required"))?;
    let buffer_status = validate_status(
        buffer,
        &format!("{portfolio_id}.buffer_recovery"),
        &["PASS", "FAIL", "UNKNOWN"],
    )?;

    let (service_totals, service_pass) =
        service_totals_report(&aggregates, context.baseline_aggregates, context.limits, &metric_by_service)?;

    let disposition = if safety_status == "VIOLATED" {
        "INFEASIBLE"
    } else if !service_pass {
        "INFEASIBLE"
    } else if safety_status == "UNKNOWN" || buffer_status == "UNKNOWN" {
        "ABSTAIN_MISSING_UPSTREAM_EVIDENCE"
    } else if safety_status != "PASS" {
        "INFEASIBLE"
    } else if buffer_status == "FAIL" {
        "MIXED_SERVICE_BUFFER_RECOVERY"
    } else if buffer_status == "PASS" {
        "FEASIBLE"
    } else {
        return Err(format!("{portfolio_id}: unsupported safety/buffer state"));
    };

    let expected = portfolio
        .get("expected_report")
        .ok_or_else(|| format!("{portfolio_id}.expected_report is required"))?;

    let computed = json!({
        "portfolio_id": portfolio_id,
        "disposition": disposition,
        "lifecycle_cost_minor_units": cost,
        "service_totals": service_totals,
        "service_outcomes": metrics_to_json(&metrics)
    });

    if &computed != expected {
        return Err(format!(
            "{portfolio_id}: computed report differs from the frozen exact expected report"
        ));
    }

    Ok(computed)
}

fn evaluate_fixture(fixture: &str, registry: &str) -> Result<Value, String> {
    let root: Value =
        serde_json::from_str(fixture).map_err(|error| format!("fixture JSON invalid: {error}"))?;
    let registry_root: Value = serde_json::from_str(registry)
        .map_err(|error| format!("threat register JSON invalid: {error}"))?;

    if required_text(&root, "schema_version", "root")? != SCHEMA {
        return Err(format!("schema_version must equal {SCHEMA}"));
    }
    if required_text(&root, "evidence_class", "root")? != "synthetic" {
        return Err("E0 reference fixture must be marked synthetic".to_owned());
    }
    reject_unsupported_numeric_risk_claims(&root, "fixture")?;

    let families = registry_root
        .get("families")
        .and_then(Value::as_array)
        .ok_or_else(|| "threat register families must be an array".to_owned())?;
    let family_ids: BTreeSet<String> = families
        .iter()
        .map(|family| required_text(family, "id", "threat_family").map(str::to_owned))
        .collect::<Result<_, _>>()?;
    if family_ids.is_empty() || family_ids.len() != families.len() {
        return Err("threat-family IDs must be non-empty and unique".to_owned());
    }

    let scope = root
        .get("scope")
        .ok_or_else(|| "root.scope is required".to_owned())?;
    let context_id = required_text(scope, "context_id", "scope")?;
    required_text(scope, "jurisdiction_context", "scope")?;
    required_text(scope, "system_boundary_id", "scope")?;
    required_text(scope, "scenario_id", "scope")?;
    if required_text(scope, "scenario_basis", "scope")?
        != "BOUNDED_DETERMINISTIC_NOT_PROBABILISTIC"
    {
        return Err("E0 must remain a bounded deterministic, non-probabilistic scenario".to_owned());
    }
    required_text(scope, "demand_profile_id", "scope")?;
    let horizon = usize::try_from(required_u64(scope, "horizon_intervals", "scope")?)
        .map_err(|_| "horizon_intervals overflow".to_owned())?;
    let interval_minutes = required_u64(scope, "interval_minutes", "scope")?;
    if horizon == 0 || interval_minutes == 0 {
        return Err("horizon and interval duration must be positive".to_owned());
    }
    required_text(scope, "currency", "scope")?;
    required_u64(scope, "minor_unit_scale", "scope")?;
    required_text(scope, "cost_basis", "scope")?;
    required_u64(scope, "price_year", "scope")?;
    required_u64(scope, "lifecycle_horizon_years", "scope")?;

    let contract = root
        .get("evaluation_contract")
        .ok_or_else(|| "evaluation_contract is required".to_owned())?;
    let sustained_intervals = usize::try_from(required_u64(
        contract,
        "sustained_recovery_intervals",
        "evaluation_contract",
    )?)
    .map_err(|_| "sustained_recovery_intervals overflow".to_owned())?;
    if sustained_intervals == 0 || sustained_intervals > horizon {
        return Err("sustained_recovery_intervals must be in 1..=horizon".to_owned());
    }
    let limits = contract
        .get("service_limits")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "service_limits must be a non-empty array".to_owned())?;

    let baseline = root
        .get("baseline")
        .ok_or_else(|| "baseline is required".to_owned())?;
    let baseline_id = required_text(baseline, "portfolio_id", "baseline")?;
    if required_text(baseline, "role", "baseline")? != "BASELINE_REFERENCE_ONLY" {
        return Err("baseline role must be BASELINE_REFERENCE_ONLY".to_owned());
    }
    if baseline.get("scope") != Some(scope) {
        return Err("ABSTAIN_INCOMPARABLE_SCOPE: baseline scope differs from evaluation scope".to_owned());
    }
    let baseline_cost = lifecycle_cost(baseline)?;
    if baseline_cost != 0 {
        return Err("E0 persistence baseline must have zero incremental intervention cost".to_owned());
    }
    let baseline_outcomes = baseline
        .get("service_outcomes")
        .and_then(Value::as_array)
        .ok_or_else(|| "baseline.service_outcomes must be an array".to_owned())?;
    let baseline_contracts = outcome_contracts(baseline_outcomes, horizon)?;
    let (baseline_metrics, baseline_aggregates) =
        evaluate_outcomes(baseline_outcomes, horizon, interval_minutes, sustained_intervals)?;
    let baseline_totals: Vec<Value> = baseline_aggregates
        .iter()
        .map(|(service_id, aggregate)| {
            json!({
                "service_id": service_id,
                "deficit_milli_beneficiary_minutes": aggregate.deficit_milli_beneficiary_minutes
            })
        })
        .collect();
    let baseline_report = json!({
        "portfolio_id": baseline_id,
        "lifecycle_cost_minor_units": baseline_cost,
        "service_totals": baseline_totals,
        "service_outcomes": metrics_to_json(&baseline_metrics)
    });
    if root.get("expected_baseline_report") != Some(&baseline_report) {
        return Err("baseline: computed metrics differ from the frozen exact reference".to_owned());
    }

    let portfolios = root
        .get("portfolios")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "portfolios must be a non-empty array".to_owned())?;
    let mut portfolio_ids = BTreeSet::new();
    let mut reports = Vec::with_capacity(portfolios.len());

    for portfolio in portfolios {
        let portfolio_id = required_text(portfolio, "portfolio_id", "portfolio")?;
        if !portfolio_ids.insert(portfolio_id.to_owned()) {
            return Err(format!("duplicate portfolio_id {portfolio_id}"));
        }
        let evaluation_context = EvaluationContext {
            baseline_contracts: &baseline_contracts,
            baseline_aggregates: &baseline_aggregates,
            scope,
            horizon,
            interval_minutes,
            sustained_intervals,
            limits,
            family_ids: &family_ids,
        };
        reports.push(evaluate_portfolio(portfolio, &evaluation_context)?);
    }

    Ok(json!({
        "schema_version": SCHEMA,
        "context_id": context_id,
        "scope_sha256": scope_digest(scope)?,
        "baseline_portfolio_id": baseline_id,
        "baseline_service_totals": baseline_totals,
        "baseline_service_outcomes": metrics_to_json(&baseline_metrics),
        "portfolio_reports": reports,
        "all_expected_reports_match": true,
        "claim_ceiling": "exact synthetic E0 evidence-accounting conformance only; no physical protection or real-world resilience claim"
    }))
}

fn run() -> Result<Value, String> {
    evaluate_fixture(FIXTURE, REGISTRY)
}

fn main() {
    match run() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("report serialization")
        ),
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("embedded fixture JSON")
    }

    #[test]
    fn e0_exact_reference_matches_all_three_portfolio_answers() {
        let report = run().expect("valid E0 reference");
        assert_eq!(report["all_expected_reports_match"], Value::Bool(true));
        assert_eq!(report["baseline_service_totals"][0]["deficit_milli_beneficiary_minutes"], 6_240_000);
        assert_eq!(report["portfolio_reports"].as_array().expect("reports").len(), 3);
    }

    #[test]
    fn baseline_golden_metrics_reject_trajectory_drift() {
        let mut root = fixture_value();
        root["baseline"]["service_outcomes"][0]["delivered_service_permille"] =
            json!([400, 500, 800, 800]);
        let error = evaluate_fixture(&root.to_string(), REGISTRY).expect_err("baseline drift");
        assert!(error.contains("baseline: computed metrics differ"));
    }

    #[test]
    fn safety_floor_remains_non_compensatory_even_at_zero_cost() {
        let mut root = fixture_value();
        root["portfolios"][2]["cost_items"][0]["present_value_minor_units"] = json!(0);
        root["portfolios"][2]["expected_report"]["lifecycle_cost_minor_units"] = json!(0);
        let report = evaluate_fixture(&root.to_string(), REGISTRY).expect("updated exact cost");
        assert_eq!(report["portfolio_reports"][2]["disposition"], "INFEASIBLE");
    }

    #[test]
    fn unsafe_low_cost_portfolio_is_infeasible_despite_zero_service_deficit() {
        let report = run().expect("valid E0 reference");
        let unsafe_report = &report["portfolio_reports"][2];
        assert_eq!(unsafe_report["disposition"], "INFEASIBLE");
        assert_eq!(unsafe_report["service_totals"][0]["deficit_milli_beneficiary_minutes"], 0);
        assert_eq!(unsafe_report["lifecycle_cost_minor_units"], 5_000_000);
    }

    #[test]
    fn service_recovery_with_depleted_buffer_is_mixed_not_feasible() {
        let report = run().expect("valid E0 reference");
        assert_eq!(
            report["portfolio_reports"][1]["disposition"],
            "MIXED_SERVICE_BUFFER_RECOVERY"
        );
    }

    #[test]
    fn unknown_buffer_evidence_abstains_without_assuming_recovery() {
        let mut root = fixture_value();
        root["portfolios"][0]["buffer_recovery"]["status"] = Value::String("UNKNOWN".into());
        root["portfolios"][0]["expected_report"]["disposition"] =
            Value::String("ABSTAIN_MISSING_UPSTREAM_EVIDENCE".into());
        let report = evaluate_fixture(&root.to_string(), REGISTRY).expect("unknown evidence abstains");
        assert_eq!(
            report["portfolio_reports"][0]["disposition"],
            "ABSTAIN_MISSING_UPSTREAM_EVIDENCE"
        );
    }

    #[test]
    fn scope_mismatch_abstains_instead_of_ranking_incomparable_portfolios() {
        let mut root = fixture_value();
        root["portfolios"][0]["scope"]["demand_profile_id"] =
            Value::String("DIFFERENT-DEMAND".into());
        let error = evaluate_fixture(&root.to_string(), REGISTRY).expect_err("scope mismatch");
        assert!(error.contains("ABSTAIN_INCOMPARABLE_SCOPE"));
    }

    #[test]
    fn duplicate_cost_identity_fails_closed() {
        let mut root = fixture_value();
        let duplicate_id = root["portfolios"][0]["cost_items"][0]["cost_id"].clone();
        root["portfolios"][0]["cost_items"][1]["cost_id"] = duplicate_id;
        let error = evaluate_fixture(&root.to_string(), REGISTRY).expect_err("duplicate cost ID");
        assert!(error.contains("duplicate cost_id"));
    }

    #[test]
    fn duplicate_benefit_claim_for_one_outcome_fails_closed() {
        let mut root = fixture_value();
        let duplicate_outcome = root["portfolios"][0]["benefit_claims"][0]["outcome_id"].clone();
        root["portfolios"][0]["benefit_claims"][1]["outcome_id"] = duplicate_outcome;
        let error = evaluate_fixture(&root.to_string(), REGISTRY).expect_err("duplicate benefit");
        assert!(error.contains("credited more than once"));
    }

    #[test]
    fn unregistered_hazard_reference_fails_closed() {
        let mut root = fixture_value();
        root["portfolios"][0]["benefit_claims"][0]["related_hazard_families"][0] =
            Value::String("UNREGISTERED-FAMILY".into());
        let error = evaluate_fixture(&root.to_string(), REGISTRY).expect_err("unknown hazard");
        assert!(error.contains("unknown hazard-family ID"));
    }

    #[test]
    fn numeric_probability_without_evidence_contract_is_rejected() {
        let mut root = fixture_value();
        root["portfolios"][0]["event_probability"] = json!(0.01);
        let error = evaluate_fixture(&root.to_string(), REGISTRY).expect_err("numeric probability");
        assert!(error.contains("without an admitted probability/loss evidence contract"));
    }

    #[test]
    fn incomplete_sustained_recovery_window_does_not_count_as_recovered() {
        let mut root = fixture_value();
        root["baseline"]["service_outcomes"][1]["delivered_service_permille"] =
            json!([200, 400, 600, 800]);
        let outcomes = root["baseline"]["service_outcomes"]
            .as_array()
            .expect("baseline outcomes");
        let (metrics, _) = evaluate_outcomes(outcomes, 4, 60, 2).expect("metrics");
        assert_eq!(metrics[1].sustained_recovery_start_minutes, None);
    }

    #[test]
    fn repeated_service_benefit_is_not_summed_across_different_service_ids() {
        let outcomes = vec![
            json!({
                "outcome_id":"WATER-1","service_id":"WATER","beneficiary_stratum":"GROUP-1",
                "beneficiary_unit":"PERSON","beneficiary_count":1,"minimum_service_permille":800,
                "delivered_service_permille":[700,800],"origin":{"kind":"SYNTHETIC_REFERENCE_TRAJECTORY",
                "upstream_owner":"test","source_receipt":"test","claim_ceiling":"test"}
            }),
            json!({
                "outcome_id":"POWER-1","service_id":"POWER","beneficiary_stratum":"GROUP-1",
                "beneficiary_unit":"PERSON","beneficiary_count":1,"minimum_service_permille":800,
                "delivered_service_permille":[700,800],"origin":{"kind":"SYNTHETIC_REFERENCE_TRAJECTORY",
                "upstream_owner":"test","source_receipt":"test","claim_ceiling":"test"}
            })
        ];
        let (_, totals) = evaluate_outcomes(&outcomes, 2, 60, 2).expect("separate service totals");
        assert_eq!(totals.len(), 2);
        assert_eq!(totals["WATER"].deficit_milli_beneficiary_minutes, 6_000);
        assert_eq!(totals["POWER"].deficit_milli_beneficiary_minutes, 6_000);
    }
}

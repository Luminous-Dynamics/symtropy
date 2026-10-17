use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::process::ExitCode;

#[derive(Clone, Copy, Debug)]
struct Provider {
    id: &'static str,
    capacity_units: u64,
}

#[derive(Clone, Copy, Debug)]
struct Portfolio {
    id: &'static str,
    lifecycle_cost_scu: u64,
    providers: &'static [Provider],
}

const DEMAND_UNITS: u64 = 70;
const CONCENTRATED_PROVIDERS: [Provider; 4] = [
    Provider { id: "provider-a", capacity_units: 50 },
    Provider { id: "provider-b", capacity_units: 20 },
    Provider { id: "provider-c", capacity_units: 15 },
    Provider { id: "provider-d", capacity_units: 15 },
];
const DIVERSIFIED_PROVIDERS: [Provider; 4] = [
    Provider { id: "provider-a", capacity_units: 25 },
    Provider { id: "provider-b", capacity_units: 25 },
    Provider { id: "provider-c", capacity_units: 25 },
    Provider { id: "provider-d", capacity_units: 25 },
];
const PORTFOLIOS: [Portfolio; 2] = [
    Portfolio {
        id: "concentrated",
        lifecycle_cost_scu: 100,
        providers: &CONCENTRATED_PROVIDERS,
    },
    Portfolio {
        id: "diversified",
        lifecycle_cost_scu: 118,
        providers: &DIVERSIFIED_PROVIDERS,
    },
];

fn validate_portfolio(portfolio: &Portfolio) -> Result<(), String> {
    if portfolio.lifecycle_cost_scu == 0 || portfolio.providers.is_empty() {
        return Err(format!("{}: cost and provider set must be non-empty", portfolio.id));
    }
    let mut ids = BTreeSet::new();
    let mut total = 0_u64;
    for provider in portfolio.providers {
        if provider.capacity_units == 0 {
            return Err(format!("{}: provider {} has zero capacity", portfolio.id, provider.id));
        }
        if !ids.insert(provider.id) {
            return Err(format!("{}: duplicate provider id {}", portfolio.id, provider.id));
        }
        total = total.checked_add(provider.capacity_units)
            .ok_or_else(|| format!("{}: total capacity overflow", portfolio.id))?;
    }
    if total != 100 {
        return Err(format!("{}: capacity shares must sum exactly to 100; found {total}", portfolio.id));
    }
    Ok(())
}

// Capacity inputs are normalized shares summing to 100. This returns the
// conventional HHI sum of squared percentage shares, without floating point.
fn hhi(portfolio: &Portfolio) -> Result<u64, String> {
    validate_portfolio(portfolio)?;
    portfolio.providers.iter().try_fold(0_u64, |sum, provider| {
        sum.checked_add(provider.capacity_units * provider.capacity_units)
            .ok_or_else(|| format!("{}: HHI arithmetic overflow", portfolio.id))
    })
}

fn evaluate(portfolio: &Portfolio, demand: u64) -> Result<Value, String> {
    validate_portfolio(portfolio)?;
    if demand == 0 {
        return Err("service demand must be positive".to_owned());
    }
    let total_capacity: u64 = portfolio.providers.iter().map(|p| p.capacity_units).sum();
    let mut faults = Vec::new();
    let mut all_single_faults_pass = true;
    let mut min_capacity = u64::MAX;
    for failed in portfolio.providers {
        let remaining = total_capacity - failed.capacity_units;
        let unmet = demand.saturating_sub(remaining);
        let pass = remaining >= demand;
        if !pass {
            all_single_faults_pass = false;
        }
        min_capacity = min_capacity.min(remaining);
        faults.push(json!({
            "fault_id": format!("supplier/{}", failed.id),
            "failed_supplier_id": failed.id,
            "remaining_capacity_units": remaining,
            "demand_units": demand,
            "unmet_demand_units": unmet,
            "status": if pass { "pass" } else { "fail" },
            "reason_code": if pass { "SERVICE_FLOOR_MET" } else { "SUPPLIER_OUTAGE_SHORTFALL" }
        }));
    }
    Ok(json!({
        "portfolio_id": portfolio.id,
        "lifecycle_cost_scu": portfolio.lifecycle_cost_scu,
        "total_capacity_units": total_capacity,
        "demand_units": demand,
        "spare_capacity_units_nominal": total_capacity.saturating_sub(demand),
        "hhi": hhi(portfolio)?,
        "minimum_remaining_capacity_under_any_single_supplier_loss": min_capacity,
        "n_minus_one_supplier_gate_pass": all_single_faults_pass,
        "fault_case_count": faults.len(),
        "fault_results": faults,
        "claim_ceiling": "Synthetic provider-capacity portfolio only; no empirical supplier market shares, outage probabilities, causal market-power conclusion or real procurement cost."
    }))
}

fn is_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn execute(source_commit: &str) -> Result<Value, String> {
    if !is_commit_sha(source_commit) {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }
    let concentrated = evaluate(&PORTFOLIOS[0], DEMAND_UNITS)?;
    let diversified = evaluate(&PORTFOLIOS[1], DEMAND_UNITS)?;
    if concentrated["hhi"] != 3350 || diversified["hhi"] != 2500 {
        return Err("HHI did not match the exact analytical oracle".to_owned());
    }
    if concentrated["n_minus_one_supplier_gate_pass"] != false
        || diversified["n_minus_one_supplier_gate_pass"] != true
    {
        return Err("single-supplier gate outcomes differ from the analytical oracle".to_owned());
    }
    if concentrated["minimum_remaining_capacity_under_any_single_supplier_loss"] != 50
        || diversified["minimum_remaining_capacity_under_any_single_supplier_loss"] != 75
    {
        return Err("minimum remaining capacity differs from the analytical oracle".to_owned());
    }
    let concentrated_cost = PORTFOLIOS[0].lifecycle_cost_scu;
    let diversified_cost = PORTFOLIOS[1].lifecycle_cost_scu;
    if diversified_cost <= concentrated_cost {
        return Err("fixture must retain a positive diversification cost premium".to_owned());
    }
    let premium_percent = (diversified_cost - concentrated_cost) * 100 / concentrated_cost;
    if premium_percent != 18 {
        return Err("diversification premium differs from the analytical oracle".to_owned());
    }

    let source_digest = format!("{:x}", Sha256::digest(include_bytes!("supplier_concentration_reference.rs")));
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "reference_version": "supplier-concentration-n-minus-one-v0.1",
        "source_commit": source_commit,
        "reference_source_sha256": source_digest,
        "result": "pass",
        "evidence_class": "synthetic_deterministic_fault_set",
        "claim_ceiling": "Synthetic capacity-share fixture only; no real launch-market estimate, cost model or outage probability.",
        "demand_units": DEMAND_UNITS,
        "hard_gate": "Require service demand after each named single-supplier failure before comparing lifecycle cost.",
        "selected_portfolio_if_n_minus_one_is_mandatory": "diversified",
        "diversification_cost_premium_percent_integer": premium_percent,
        "portfolios": [concentrated, diversified]
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err("usage: cargo run --example supplier_concentration_reference -- --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    println!("{}", serde_json::to_string_pretty(&execute(&args[2])?).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("supplier_concentration_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hhi_matches_exact_synthetic_oracle() {
        assert_eq!(hhi(&PORTFOLIOS[0]).unwrap(), 3350);
        assert_eq!(hhi(&PORTFOLIOS[1]).unwrap(), 2500);
    }

    #[test]
    fn concentrated_portfolio_fails_only_the_largest_provider_loss() {
        let report = evaluate(&PORTFOLIOS[0], DEMAND_UNITS).unwrap();
        assert_eq!(report["n_minus_one_supplier_gate_pass"], false);
        assert_eq!(report["minimum_remaining_capacity_under_any_single_supplier_loss"], 50);
        let failures = report["fault_results"].as_array().unwrap()
            .iter().filter(|r| r["status"] == "fail").count();
        assert_eq!(failures, 1);
        assert_eq!(report["fault_results"][0]["unmet_demand_units"], 20);
    }

    #[test]
    fn diversified_portfolio_passes_each_single_supplier_loss_at_demand_seventy() {
        let report = evaluate(&PORTFOLIOS[1], DEMAND_UNITS).unwrap();
        assert_eq!(report["n_minus_one_supplier_gate_pass"], true);
        assert_eq!(report["minimum_remaining_capacity_under_any_single_supplier_loss"], 75);
        let faults = report["fault_results"].as_array().unwrap();
        assert_eq!(faults.len(), 4);
        assert!(faults.iter().all(|fault| fault["status"] == "pass"));
    }

    #[test]
    fn robustness_is_not_claimed_when_demand_exceeds_remaining_capacity() {
        let report = evaluate(&PORTFOLIOS[1], 76).unwrap();
        assert_eq!(report["n_minus_one_supplier_gate_pass"], false);
        assert_eq!(report["minimum_remaining_capacity_under_any_single_supplier_loss"], 75);
    }

    #[test]
    fn invalid_share_sum_and_duplicate_provider_fail_closed() {
        let invalid = [Provider { id: "x", capacity_units: 99 }];
        let invalid_portfolio = Portfolio { id: "bad", lifecycle_cost_scu: 1, providers: &invalid };
        assert!(validate_portfolio(&invalid_portfolio).is_err());

        let duplicate = [
            Provider { id: "x", capacity_units: 50 },
            Provider { id: "x", capacity_units: 50 },
        ];
        let duplicate_portfolio = Portfolio { id: "duplicate", lifecycle_cost_scu: 1, providers: &duplicate };
        assert!(validate_portfolio(&duplicate_portfolio).is_err());
    }

    #[test]
    fn rejects_invalid_source_commit_before_receipt() {
        assert!(execute("not-a-commit-sha").is_err());
    }
}

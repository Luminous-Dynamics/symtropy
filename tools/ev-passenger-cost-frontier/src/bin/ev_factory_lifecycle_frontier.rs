use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const BENCHMARK_ID: &str = "EV-FACTORY-LIFECYCLE-001";
const MIN_FAMILIES: usize = 5;
const CLAIM_CEILING: &str = "Synthetic equipment lifecycle-cost sensitivity only. No values are market quotes, measured plant costs, bankable ROI, SOTA evidence or authorization for factory/product/safety release.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Catalog {
    schema_version: String,
    benchmark_id: String,
    evidence_class: EvidenceClass,
    cost_unit: String,
    time_unit: String,
    discount_rate_per_year: f64,
    families: Vec<FamilyProfile>,
    strategies: Vec<Strategy>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceClass {
    Synthetic,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct FamilyProfile {
    family_id: String,
    planning_horizon_years: u32,
    active_fraction: f64,
    product_change_events_per_year: f64,
    reuse_success_fraction: f64,
    redeployability_fraction: f64,
    salvage_realization_fraction: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Strategy {
    strategy_id: String,
    kind: StrategyKind,
    owns_asset: bool,
    acquisition_cost: f64,
    refurbishment_cost: f64,
    installation_cost: f64,
    annual_contract_cost: f64,
    contract_payment_basis: ContractPaymentBasis,
    annual_maintenance_cost: f64,
    annual_idle_carry_cost: f64,
    retooling_cost_per_product_change: f64,
    fallback_replacement_cost_if_reuse_fails: f64,
    terminal_salvage_value: f64,
    terminal_disposal_cost: f64,
    note: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum StrategyKind {
    PurchaseNew,
    Lease,
    ShortTermRent,
    RefurbishRedeploy,
}

impl StrategyKind {
    const ALL: [Self; 4] = [
        Self::PurchaseNew,
        Self::Lease,
        Self::ShortTermRent,
        Self::RefurbishRedeploy,
    ];

    fn owns_asset(self) -> bool {
        matches!(self, Self::PurchaseNew | Self::RefurbishRedeploy)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ContractPaymentBasis {
    FixedHorizon,
    ActiveFraction,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct FamilyStrategyResult {
    family_id: String,
    strategy_id: String,
    strategy_kind: StrategyKind,
    source_cost_evidence_class: EvidenceClass,
    planning_horizon_years: u32,
    discount_rate_per_year: f64,
    upfront_npv_cost: f64,
    contract_payment_npv_cost: f64,
    maintenance_npv_cost: f64,
    idle_carry_npv_cost: f64,
    retooling_npv_cost: f64,
    expected_reuse_failure_npv_cost: f64,
    terminal_salvage_credit_npv: f64,
    terminal_disposal_npv_cost: f64,
    total_npv_cost: f64,
    stranded_asset_exposure: f64,
    effective_recovery_fraction: f64,
    family_best_strategy_id: String,
    regret_vs_family_best_npv_cost: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct StrategyRobustScore {
    strategy_id: String,
    strategy_kind: StrategyKind,
    mean_npv_cost_across_families: f64,
    worst_family_npv_cost: f64,
    best_family_npv_cost: f64,
    mean_regret_npv_cost: f64,
    maximum_regret_npv_cost: f64,
    mean_stranded_asset_exposure: f64,
    maximum_stranded_asset_exposure: f64,
    families_where_strategy_is_cost_best: usize,
    family_count: usize,
    pareto_nondominated: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct BreakEvenResult {
    family_id: String,
    purchase_strategy_id: String,
    lease_strategy_id: String,
    planning_horizon_cap_years: u32,
    first_year_purchase_npv_not_above_lease_npv: Option<u32>,
    purchase_npv_at_cap_years: f64,
    lease_npv_at_cap_years: f64,
    note: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct SensitivityResult {
    case_id: String,
    perturbed_dimension: String,
    perturbation_description: String,
    mean_npv_cost_by_strategy: BTreeMap<String, f64>,
    worst_family_npv_cost_by_strategy: BTreeMap<String, f64>,
    cost_best_strategy_by_mean_npv: String,
    evidence_class: EvidenceClass,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    source_catalog_sha256: String,
    source_evidence_class: EvidenceClass,
    cost_unit: String,
    time_unit: String,
    discount_rate_per_year: f64,
    family_count: usize,
    family_ids: Vec<String>,
    strategy_count: usize,
    strategy_ids: Vec<String>,
    family_strategy_results: Vec<FamilyStrategyResult>,
    strategy_scores: Vec<StrategyRobustScore>,
    pareto_frontier_strategy_ids: Vec<String>,
    purchase_vs_lease_break_even: Vec<BreakEvenResult>,
    sensitivity_results: Vec<SensitivityResult>,
    family_aggregation_note: String,
    economic_recommendation_authorized: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

#[derive(Debug, Clone)]
struct CostBreakdown {
    upfront: f64,
    contract: f64,
    maintenance: f64,
    idle_carry: f64,
    retooling: f64,
    reuse_failure: f64,
    salvage_credit: f64,
    disposal: f64,
    total: f64,
    stranded_asset_exposure: f64,
    effective_recovery_fraction: f64,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: ev_factory_lifecycle_frontier <versioned-catalog.json>");
        process::exit(2);
    }
    let raw = match fs::read_to_string(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read catalog {}: {error}", args[1]);
            process::exit(2);
        }
    };
    let catalog: Catalog = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid lifecycle catalog: {error}");
            process::exit(2);
        }
    };
    if let Err(error) = validate_catalog(&catalog) {
        eprintln!("lifecycle catalog rejected: {error}");
        process::exit(2);
    }
    let digest = format!("{:x}", Sha256::digest(raw.as_bytes()));
    let report = match evaluate(&catalog, &digest) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("lifecycle frontier failed: {error}");
            process::exit(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize lifecycle report: {error}");
            process::exit(2);
        }
    }
}

fn valid_fraction(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{name}_must_be_finite_and_between_zero_and_one"));
    }
    Ok(())
}

fn valid_nonnegative(name: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{name}_must_be_finite_and_nonnegative"));
    }
    Ok(())
}

fn validate_catalog(catalog: &Catalog) -> Result<(), String> {
    if catalog.schema_version != "0.1.0"
        || catalog.benchmark_id != BENCHMARK_ID
        || catalog.evidence_class != EvidenceClass::Synthetic
        || catalog.cost_unit.trim().is_empty()
        || catalog.time_unit != "year"
        || catalog.claim_ceiling.trim().is_empty()
    {
        return Err("unsupported_schema_benchmark_evidence_or_units".into());
    }
    if !catalog.discount_rate_per_year.is_finite()
        || !(0.0..=0.5).contains(&catalog.discount_rate_per_year)
    {
        return Err("discount_rate_per_year_must_be_between_zero_and_point_five".into());
    }
    if catalog.families.len() < MIN_FAMILIES {
        return Err(format!(
            "at_least_{MIN_FAMILIES}_product_evolution_families_required"
        ));
    }
    if catalog.strategies.len() != StrategyKind::ALL.len() {
        return Err("exactly_four_declared_lifecycle_strategies_required".into());
    }
    let mut family_ids = BTreeSet::new();
    for family in &catalog.families {
        if family.family_id.trim().is_empty() || !family_ids.insert(family.family_id.clone()) {
            return Err("family_ids_must_be_nonempty_and_unique".into());
        }
        if family.planning_horizon_years == 0 || family.planning_horizon_years > 30 {
            return Err("planning_horizon_must_be_between_one_and_thirty_years".into());
        }
        valid_fraction("active_fraction", family.active_fraction)?;
        valid_fraction("reuse_success_fraction", family.reuse_success_fraction)?;
        valid_fraction("redeployability_fraction", family.redeployability_fraction)?;
        valid_fraction(
            "salvage_realization_fraction",
            family.salvage_realization_fraction,
        )?;
        valid_nonnegative(
            "product_change_events_per_year",
            family.product_change_events_per_year,
        )?;
    }

    let mut strategy_ids = BTreeSet::new();
    let mut strategy_kinds = BTreeSet::new();
    for strategy in &catalog.strategies {
        if strategy.strategy_id.trim().is_empty()
            || !strategy_ids.insert(strategy.strategy_id.clone())
            || !strategy_kinds.insert(strategy.kind)
            || strategy.note.trim().is_empty()
        {
            return Err("strategy_ids_and_kinds_must_be_nonempty_and_unique".into());
        }
        if strategy.owns_asset != strategy.kind.owns_asset() {
            return Err(format!(
                "strategy_{}_ownership_flag_conflicts_with_kind",
                strategy.strategy_id
            ));
        }
        for (name, value) in [
            ("acquisition_cost", strategy.acquisition_cost),
            ("refurbishment_cost", strategy.refurbishment_cost),
            ("installation_cost", strategy.installation_cost),
            ("annual_contract_cost", strategy.annual_contract_cost),
            ("annual_maintenance_cost", strategy.annual_maintenance_cost),
            ("annual_idle_carry_cost", strategy.annual_idle_carry_cost),
            (
                "retooling_cost_per_product_change",
                strategy.retooling_cost_per_product_change,
            ),
            (
                "fallback_replacement_cost_if_reuse_fails",
                strategy.fallback_replacement_cost_if_reuse_fails,
            ),
            ("terminal_salvage_value", strategy.terminal_salvage_value),
            ("terminal_disposal_cost", strategy.terminal_disposal_cost),
        ] {
            valid_nonnegative(name, value)?;
        }
        if !strategy.owns_asset
            && (strategy.acquisition_cost > 0.0
                || strategy.refurbishment_cost > 0.0
                || strategy.annual_idle_carry_cost > 0.0
                || strategy.terminal_salvage_value > 0.0
                || strategy.fallback_replacement_cost_if_reuse_fails > 0.0)
        {
            return Err(format!(
                "nonowning_strategy_{}_has_asset_only_cost_fields",
                strategy.strategy_id
            ));
        }
        if strategy.kind != StrategyKind::RefurbishRedeploy
            && strategy.fallback_replacement_cost_if_reuse_fails > 0.0
        {
            return Err(format!(
                "strategy_{}_has_reuse_failure_fallback_without_reuse",
                strategy.strategy_id
            ));
        }
        if strategy.kind == StrategyKind::ShortTermRent
            && strategy.contract_payment_basis != ContractPaymentBasis::ActiveFraction
        {
            return Err("short_term_rent_must_charge_only_active_fraction".into());
        }
        if strategy.kind == StrategyKind::Lease
            && strategy.contract_payment_basis != ContractPaymentBasis::FixedHorizon
        {
            return Err("lease_must_use_fixed_horizon_payment_basis".into());
        }
    }
    if strategy_kinds != StrategyKind::ALL.into_iter().collect() {
        return Err("lifecycle_strategy_kind_set_incomplete".into());
    }
    let purchase = catalog
        .strategies
        .iter()
        .find(|s| s.kind == StrategyKind::PurchaseNew)
        .unwrap();
    let lease = catalog
        .strategies
        .iter()
        .find(|s| s.kind == StrategyKind::Lease)
        .unwrap();
    if purchase.acquisition_cost <= 0.0 || lease.annual_contract_cost <= 0.0 {
        return Err(
            "purchase_cost_and_lease_payment_must_be_positive_for_break_even_analysis".into(),
        );
    }
    Ok(())
}

fn discount(rate: f64, year: u32) -> f64 {
    1.0 / (1.0 + rate).powi(year as i32)
}

fn calculate_cost(
    strategy: &Strategy,
    family: &FamilyProfile,
    discount_rate: f64,
    horizon_override: Option<u32>,
) -> CostBreakdown {
    let horizon = horizon_override.unwrap_or(family.planning_horizon_years);
    let active_fraction = family.active_fraction;
    let idle_fraction = 1.0 - active_fraction;
    let expected_reuse_failure = if strategy.kind == StrategyKind::RefurbishRedeploy {
        strategy.fallback_replacement_cost_if_reuse_fails * (1.0 - family.reuse_success_fraction)
    } else {
        0.0
    };
    let upfront =
        strategy.acquisition_cost + strategy.refurbishment_cost + strategy.installation_cost;
    let mut contract = 0.0;
    let mut maintenance = 0.0;
    let mut idle_carry = 0.0;
    let mut retooling = 0.0;
    for year in 1..=horizon {
        let df = discount(discount_rate, year);
        let contract_scale = match strategy.contract_payment_basis {
            ContractPaymentBasis::FixedHorizon => 1.0,
            ContractPaymentBasis::ActiveFraction => active_fraction,
        };
        contract += strategy.annual_contract_cost * contract_scale * df;
        maintenance += strategy.annual_maintenance_cost * active_fraction * df;
        idle_carry += strategy.annual_idle_carry_cost * idle_fraction * df;
        retooling +=
            strategy.retooling_cost_per_product_change * family.product_change_events_per_year * df;
    }
    let effective_recovery_fraction = if strategy.owns_asset {
        family.salvage_realization_fraction
            * family.redeployability_fraction
            * family.reuse_success_fraction
    } else {
        0.0
    };
    let salvage_credit = if strategy.owns_asset {
        strategy.terminal_salvage_value
            * effective_recovery_fraction
            * discount(discount_rate, horizon)
    } else {
        0.0
    };
    let disposal = strategy.terminal_disposal_cost
        * (if strategy.owns_asset {
            1.0 - effective_recovery_fraction
        } else {
            1.0
        })
        * discount(discount_rate, horizon);
    let total = upfront
        + expected_reuse_failure
        + contract
        + maintenance
        + idle_carry
        + retooling
        + disposal
        - salvage_credit;
    let owned_investment = upfront + expected_reuse_failure;
    let stranded_asset_exposure = if strategy.owns_asset {
        owned_investment * (1.0 - effective_recovery_fraction)
    } else {
        0.0
    };
    CostBreakdown {
        upfront,
        contract,
        maintenance,
        idle_carry,
        retooling,
        reuse_failure: expected_reuse_failure,
        salvage_credit,
        disposal,
        total,
        stranded_asset_exposure,
        effective_recovery_fraction,
    }
}

fn evaluate(catalog: &Catalog, source_sha256: &str) -> Result<Report, String> {
    if source_sha256.len() != 64 || !source_sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("source_catalog_sha256_malformed".into());
    }
    validate_catalog(catalog)?;
    let mut families = catalog.families.clone();
    families.sort_by(|a, b| a.family_id.cmp(&b.family_id));
    let mut strategies = catalog.strategies.clone();
    strategies.sort_by(|a, b| a.strategy_id.cmp(&b.strategy_id));
    let mut results = Vec::new();
    let mut family_best = BTreeMap::<String, (String, f64)>::new();
    let mut computed = BTreeMap::<(String, String), CostBreakdown>::new();
    for family in &families {
        for strategy in &strategies {
            let cost = calculate_cost(strategy, family, catalog.discount_rate_per_year, None);
            computed.insert(
                (family.family_id.clone(), strategy.strategy_id.clone()),
                cost.clone(),
            );
            let entry = family_best
                .entry(family.family_id.clone())
                .or_insert((strategy.strategy_id.clone(), cost.total));
            if cost.total < entry.1 {
                *entry = (strategy.strategy_id.clone(), cost.total);
            }
        }
    }
    for family in &families {
        let (family_best_id, family_best_cost) = &family_best[&family.family_id];
        for strategy in &strategies {
            let cost = &computed[&(family.family_id.clone(), strategy.strategy_id.clone())];
            results.push(FamilyStrategyResult {
                family_id: family.family_id.clone(),
                strategy_id: strategy.strategy_id.clone(),
                strategy_kind: strategy.kind,
                source_cost_evidence_class: catalog.evidence_class,
                planning_horizon_years: family.planning_horizon_years,
                discount_rate_per_year: catalog.discount_rate_per_year,
                upfront_npv_cost: cost.upfront,
                contract_payment_npv_cost: cost.contract,
                maintenance_npv_cost: cost.maintenance,
                idle_carry_npv_cost: cost.idle_carry,
                retooling_npv_cost: cost.retooling,
                expected_reuse_failure_npv_cost: cost.reuse_failure,
                terminal_salvage_credit_npv: cost.salvage_credit,
                terminal_disposal_npv_cost: cost.disposal,
                total_npv_cost: cost.total,
                stranded_asset_exposure: cost.stranded_asset_exposure,
                effective_recovery_fraction: cost.effective_recovery_fraction,
                family_best_strategy_id: family_best_id.clone(),
                regret_vs_family_best_npv_cost: (cost.total - family_best_cost).max(0.0),
            });
        }
    }

    let mut scores = Vec::new();
    for strategy in &strategies {
        let rows: Vec<&FamilyStrategyResult> = results
            .iter()
            .filter(|row| row.strategy_id == strategy.strategy_id)
            .collect();
        let costs: Vec<f64> = rows.iter().map(|row| row.total_npv_cost).collect();
        let regrets: Vec<f64> = rows
            .iter()
            .map(|row| row.regret_vs_family_best_npv_cost)
            .collect();
        let exposure: Vec<f64> = rows.iter().map(|row| row.stranded_asset_exposure).collect();
        scores.push(StrategyRobustScore {
            strategy_id: strategy.strategy_id.clone(),
            strategy_kind: strategy.kind,
            mean_npv_cost_across_families: costs.iter().sum::<f64>() / costs.len() as f64,
            worst_family_npv_cost: costs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            best_family_npv_cost: costs.iter().copied().fold(f64::INFINITY, f64::min),
            mean_regret_npv_cost: regrets.iter().sum::<f64>() / regrets.len() as f64,
            maximum_regret_npv_cost: regrets.iter().copied().fold(0.0, f64::max),
            mean_stranded_asset_exposure: exposure.iter().sum::<f64>() / exposure.len() as f64,
            maximum_stranded_asset_exposure: exposure.iter().copied().fold(0.0, f64::max),
            families_where_strategy_is_cost_best: family_best
                .values()
                .filter(|(winner, _)| winner == &strategy.strategy_id)
                .count(),
            family_count: families.len(),
            pareto_nondominated: false,
        });
    }
    let mut frontier_ids = Vec::new();
    for i in 0..scores.len() {
        let dominated = (0..scores.len()).any(|j| {
            if i == j {
                return false;
            }
            let a = &scores[j];
            let b = &scores[i];
            let no_worse = a.mean_npv_cost_across_families <= b.mean_npv_cost_across_families
                && a.worst_family_npv_cost <= b.worst_family_npv_cost
                && a.maximum_stranded_asset_exposure <= b.maximum_stranded_asset_exposure;
            let strictly_better = a.mean_npv_cost_across_families < b.mean_npv_cost_across_families
                || a.worst_family_npv_cost < b.worst_family_npv_cost
                || a.maximum_stranded_asset_exposure < b.maximum_stranded_asset_exposure;
            no_worse && strictly_better
        });
        scores[i].pareto_nondominated = !dominated;
        if !dominated {
            frontier_ids.push(scores[i].strategy_id.clone());
        }
    }

    let purchase = strategies
        .iter()
        .find(|s| s.kind == StrategyKind::PurchaseNew)
        .unwrap();
    let lease = strategies
        .iter()
        .find(|s| s.kind == StrategyKind::Lease)
        .unwrap();
    let mut break_even_rows = Vec::new();
    for family in &families {
        let mut first_year = None;
        let mut last_buy = 0.0;
        let mut last_lease = 0.0;
        for years in 1..=family.planning_horizon_years {
            last_buy = calculate_cost(
                purchase,
                family,
                catalog.discount_rate_per_year,
                Some(years),
            )
            .total;
            last_lease =
                calculate_cost(lease, family, catalog.discount_rate_per_year, Some(years)).total;
            if first_year.is_none() && last_buy <= last_lease {
                first_year = Some(years);
            }
        }
        break_even_rows.push(BreakEvenResult {
            family_id: family.family_id.clone(),
            purchase_strategy_id: purchase.strategy_id.clone(),
            lease_strategy_id: lease.strategy_id.clone(),
            planning_horizon_cap_years: family.planning_horizon_years,
            first_year_purchase_npv_not_above_lease_npv: first_year,
            purchase_npv_at_cap_years: last_buy,
            lease_npv_at_cap_years: last_lease,
            note: "Earliest integer tenure at which modeled purchase NPV is no higher than modeled lease NPV, with profile active fraction and product-change rate held fixed. Synthetic assumption only.".into(),
        });
    }
    let sensitivity_results = build_sensitivity(catalog, &strategies)?;
    Ok(Report {
        schema_version: "0.1.0".into(),
        benchmark_id: BENCHMARK_ID.into(),
        source_catalog_sha256: source_sha256.to_ascii_lowercase(),
        source_evidence_class: catalog.evidence_class,
        cost_unit: catalog.cost_unit.clone(),
        time_unit: catalog.time_unit.clone(),
        discount_rate_per_year: catalog.discount_rate_per_year,
        family_count: families.len(),
        family_ids: families.iter().map(|family| family.family_id.clone()).collect(),
        strategy_count: strategies.len(),
        strategy_ids: strategies.iter().map(|strategy| strategy.strategy_id.clone()).collect(),
        family_strategy_results: results,
        strategy_scores: scores,
        pareto_frontier_strategy_ids: frontier_ids,
        purchase_vs_lease_break_even: break_even_rows,
        sensitivity_results,
        family_aggregation_note: "Product-evolution families are distinct model assumptions, not IID observations. Family NPVs are shown individually; equal-weight family averages, worst-family costs and maximum regret are separate summaries. No market or plant costs are inferred.".into(),
        economic_recommendation_authorized: false,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

fn build_sensitivity(
    catalog: &Catalog,
    strategies: &[Strategy],
) -> Result<Vec<SensitivityResult>, String> {
    let base = catalog.clone();
    let mut cases: Vec<(String, String, String, Catalog)> = Vec::new();
    for rate in [
        0.0,
        catalog.discount_rate_per_year,
        (catalog.discount_rate_per_year + 0.05).min(0.5),
    ] {
        let mut changed = base.clone();
        changed.discount_rate_per_year = rate;
        cases.push((
            format!("discount-rate-{rate:.3}"),
            "discount_rate".into(),
            format!("annual discount rate set to {rate:.3}"),
            changed,
        ));
    }
    {
        let mut changed = base.clone();
        for family in &mut changed.families {
            family.planning_horizon_years = (family.planning_horizon_years / 2).max(1);
        }
        cases.push((
            "useful-life-short".into(),
            "useful_life".into(),
            "each family horizon reduced to half, rounded down and bounded at one year".into(),
            changed,
        ));
    }
    {
        let mut changed = base.clone();
        for family in &mut changed.families {
            family.planning_horizon_years = (family.planning_horizon_years + 2).min(30);
        }
        cases.push((
            "useful-life-long".into(),
            "useful_life".into(),
            "each family horizon extended by two years, capped at thirty".into(),
            changed,
        ));
    }
    {
        let mut changed = base.clone();
        for family in &mut changed.families {
            family.product_change_events_per_year = 0.0;
        }
        cases.push((
            "product-changes-none".into(),
            "product_evolution".into(),
            "no product-change events".into(),
            changed,
        ));
    }
    {
        let mut changed = base.clone();
        for family in &mut changed.families {
            family.product_change_events_per_year *= 2.0;
        }
        cases.push((
            "product-changes-double".into(),
            "product_evolution".into(),
            "product-change frequency doubled in every family".into(),
            changed,
        ));
    }
    {
        let mut changed = base.clone();
        for family in &mut changed.families {
            family.active_fraction = 1.0;
        }
        cases.push((
            "idle-duration-none".into(),
            "idle_duration".into(),
            "all assets assumed active for the full planning horizon".into(),
            changed,
        ));
    }
    {
        let mut changed = base.clone();
        for family in &mut changed.families {
            family.active_fraction = (family.active_fraction - 0.3).max(0.0);
        }
        cases.push((
            "idle-duration-plus-30pct".into(),
            "idle_duration".into(),
            "asset active fraction reduced by 0.30, floored at zero".into(),
            changed,
        ));
    }
    {
        let mut changed = base.clone();
        for family in &mut changed.families {
            family.salvage_realization_fraction = 0.0;
        }
        cases.push((
            "salvage-realization-none".into(),
            "salvage_realization".into(),
            "no terminal salvage value is realized".into(),
            changed,
        ));
    }
    {
        let mut changed = base.clone();
        for family in &mut changed.families {
            family.salvage_realization_fraction = 1.0;
        }
        cases.push((
            "salvage-realization-full".into(),
            "salvage_realization".into(),
            "all declared terminal salvage value is realized, subject to redeployability and reuse success".into(),
            changed,
        ));
    }

    let mut results = Vec::new();
    for (case_id, dimension, description, changed) in cases {
        let mut costs_by_strategy: BTreeMap<String, Vec<f64>> = strategies
            .iter()
            .map(|strategy| (strategy.strategy_id.clone(), Vec::new()))
            .collect();
        for family in &changed.families {
            for strategy in strategies {
                costs_by_strategy
                    .get_mut(&strategy.strategy_id)
                    .unwrap()
                    .push(
                        calculate_cost(strategy, family, changed.discount_rate_per_year, None)
                            .total,
                    );
            }
        }
        let mean_cost: BTreeMap<String, f64> = costs_by_strategy
            .iter()
            .map(|(strategy, costs)| {
                (
                    strategy.clone(),
                    costs.iter().sum::<f64>() / costs.len() as f64,
                )
            })
            .collect();
        let worst_cost: BTreeMap<String, f64> = costs_by_strategy
            .iter()
            .map(|(strategy, costs)| {
                (
                    strategy.clone(),
                    costs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                )
            })
            .collect();
        let best = mean_cost
            .iter()
            .min_by(|a, b| a.1.total_cmp(b.1).then_with(|| a.0.cmp(b.0)))
            .ok_or_else(|| "sensitivity_case_has_no_strategies".to_string())?
            .0
            .clone();
        results.push(SensitivityResult {
            case_id,
            perturbed_dimension: dimension,
            perturbation_description: description,
            mean_npv_cost_by_strategy: mean_cost,
            worst_family_npv_cost_by_strategy: worst_cost,
            cost_best_strategy_by_mean_npv: best,
            evidence_class: changed.evidence_class,
        });
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn family() -> FamilyProfile {
        FamilyProfile {
            family_id: "test".into(),
            planning_horizon_years: 2,
            active_fraction: 1.0,
            product_change_events_per_year: 0.0,
            reuse_success_fraction: 1.0,
            redeployability_fraction: 1.0,
            salvage_realization_fraction: 1.0,
        }
    }

    fn base_strategy() -> Strategy {
        Strategy {
            strategy_id: "buy".into(),
            kind: StrategyKind::PurchaseNew,
            owns_asset: true,
            acquisition_cost: 100.0,
            refurbishment_cost: 0.0,
            installation_cost: 0.0,
            annual_contract_cost: 0.0,
            contract_payment_basis: ContractPaymentBasis::FixedHorizon,
            annual_maintenance_cost: 10.0,
            annual_idle_carry_cost: 0.0,
            retooling_cost_per_product_change: 0.0,
            fallback_replacement_cost_if_reuse_fails: 0.0,
            terminal_salvage_value: 20.0,
            terminal_disposal_cost: 0.0,
            note: "synthetic test".into(),
        }
    }

    #[test]
    fn present_value_and_salvage_are_applied_once() {
        let result = calculate_cost(&base_strategy(), &family(), 0.0, None);
        assert!((result.total - 100.0).abs() < 1e-12);
        assert_eq!(result.upfront, 100.0);
        assert_eq!(result.maintenance, 20.0);
        assert_eq!(result.salvage_credit, 20.0);
    }

    #[test]
    fn reuse_failure_fallback_is_reported_separately_and_counted_once() {
        let mut family = family();
        family.reuse_success_fraction = 0.5;
        let mut strategy = base_strategy();
        strategy.strategy_id = "reuse".into();
        strategy.kind = StrategyKind::RefurbishRedeploy;
        strategy.acquisition_cost = 20.0;
        strategy.refurbishment_cost = 10.0;
        strategy.installation_cost = 5.0;
        strategy.fallback_replacement_cost_if_reuse_fails = 80.0;
        let result = calculate_cost(&strategy, &family, 0.0, None);
        assert_eq!(result.upfront, 35.0);
        assert_eq!(result.reuse_failure, 40.0);
        assert_eq!(result.total, 85.0);
        let reconstructed = result.upfront
            + result.reuse_failure
            + result.contract
            + result.maintenance
            + result.idle_carry
            + result.retooling
            + result.disposal
            - result.salvage_credit;
        assert!((result.total - reconstructed).abs() < 1e-12);
    }

    #[test]
    fn unused_owned_asset_has_idle_carry_and_stranding_exposure() {
        let mut f = family();
        f.active_fraction = 0.0;
        f.redeployability_fraction = 0.0;
        let mut strategy = base_strategy();
        strategy.annual_idle_carry_cost = 5.0;
        let result = calculate_cost(&strategy, &f, 0.0, None);
        assert_eq!(result.idle_carry, 10.0);
        assert_eq!(result.maintenance, 0.0);
        assert_eq!(result.salvage_credit, 0.0);
        assert_eq!(result.stranded_asset_exposure, 100.0);
        assert_eq!(result.total, 110.0);
    }

    #[test]
    fn active_fraction_rent_does_not_pay_for_idle_years() {
        let mut f = family();
        f.active_fraction = 0.5;
        let mut rent = base_strategy();
        rent.strategy_id = "rent".into();
        rent.kind = StrategyKind::ShortTermRent;
        rent.owns_asset = false;
        rent.acquisition_cost = 0.0;
        rent.annual_maintenance_cost = 0.0;
        rent.terminal_salvage_value = 0.0;
        rent.annual_contract_cost = 100.0;
        rent.contract_payment_basis = ContractPaymentBasis::ActiveFraction;
        let result = calculate_cost(&rent, &f, 0.0, None);
        assert_eq!(result.contract, 100.0);
    }

    fn catalog_fixture() -> Catalog {
        serde_json::from_str(include_str!(
            "../../fixtures/ev_factory_lifecycle_v0_1.json"
        ))
        .expect("lifecycle fixture parses")
    }

    #[test]
    fn unknown_fields_and_inconsistent_strategy_kind_are_rejected() {
        let json = serde_json::json!({"unexpected": true});
        assert!(serde_json::from_value::<Catalog>(json).is_err());
        let mut catalog = catalog_fixture();
        catalog.strategies[0].kind = StrategyKind::Lease;
        assert!(validate_catalog(&catalog).is_err());
    }

    #[test]
    fn five_family_four_strategy_frontier_is_reproducible_and_fail_closed() {
        let catalog = catalog_fixture();
        validate_catalog(&catalog).expect("valid synthetic catalog");
        let digest = "a".repeat(64);
        let first = evaluate(&catalog, &digest).expect("frontier report");
        let second = evaluate(&catalog, &digest).expect("repeat frontier report");
        assert_eq!(first, second);
        assert_eq!(first.family_count, 5);
        assert_eq!(first.strategy_count, 4);
        assert_eq!(first.family_strategy_results.len(), 20);
        assert_eq!(first.purchase_vs_lease_break_even.len(), 5);
        assert_eq!(first.sensitivity_results.len(), 11);
        assert!(!first.economic_recommendation_authorized);
        assert!(!first.production_release_authorized);
        assert_eq!(first.source_catalog_sha256, digest);
    }

    #[test]
    fn sensitivity_grid_separates_lifecycle_dimensions() {
        let report = evaluate(&catalog_fixture(), &"b".repeat(64)).expect("report");
        let cases: BTreeSet<String> = report
            .sensitivity_results
            .iter()
            .map(|case| case.perturbed_dimension.clone())
            .collect();
        for dimension in [
            "discount_rate",
            "useful_life",
            "product_evolution",
            "idle_duration",
            "salvage_realization",
        ] {
            assert!(cases.contains(dimension));
        }
        assert!(report.sensitivity_results.iter().all(|case| {
            case.mean_npv_cost_by_strategy.len() == 4
                && case.worst_family_npv_cost_by_strategy.len() == 4
                && case.evidence_class == EvidenceClass::Synthetic
        }));
    }
}

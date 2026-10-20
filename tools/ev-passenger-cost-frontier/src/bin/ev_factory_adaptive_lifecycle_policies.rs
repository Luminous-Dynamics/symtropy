use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const BENCHMARK_ID: &str = "EV-FACTORY-OPTIONS-001";
const LOW_ACTIVITY_THRESHOLD: f64 = 0.30;
const HIGH_CHANGE_THRESHOLD: f64 = 0.70;
const REUSE_SUCCESS_THRESHOLD: f64 = 0.90;
const REDEPLOYABILITY_THRESHOLD: f64 = 0.80;
const CLAIM_CEILING: &str = "Synthetic value-of-flexibility benchmark only. Adaptive policies use declared observable-signal rules with synthetic decision overheads; they do not establish real-world ROI, calibrated strategy probabilities, SOTA, or factory/product/safety release.";

#[derive(Debug, Clone)]
struct FamilySignal {
    family_id: String,
    planning_horizon_years: u32,
    active_fraction: f64,
    product_change_events_per_year: f64,
    reuse_success_fraction: f64,
    redeployability_fraction: f64,
    salvage_realization_fraction: f64,
}

#[derive(Debug, Clone)]
struct StrategyDefinition {
    strategy_id: String,
    kind: String,
}

#[derive(Debug, Clone)]
struct CatalogData {
    source_sha256: String,
    cost_unit: String,
    discount_rate_per_year: f64,
    family_signals: Vec<FamilySignal>,
    strategies: Vec<StrategyDefinition>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum PolicyKind {
    FixedPurchase,
    FixedLease,
    FixedRent,
    FixedRefurbish,
    AdaptiveLowActivityRent,
    AdaptiveHighChangeLease,
    AdaptiveReuseConfidence,
    AdaptiveCombined,
    PerfectInformationOracle,
}

impl PolicyKind {
    const ALL: [Self; 9] = [
        Self::FixedPurchase,
        Self::FixedLease,
        Self::FixedRent,
        Self::FixedRefurbish,
        Self::AdaptiveLowActivityRent,
        Self::AdaptiveHighChangeLease,
        Self::AdaptiveReuseConfidence,
        Self::AdaptiveCombined,
        Self::PerfectInformationOracle,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::FixedPurchase => "fixed_purchase_new",
            Self::FixedLease => "fixed_long_lease",
            Self::FixedRent => "fixed_short_term_rent",
            Self::FixedRefurbish => "fixed_refurbish_redeploy",
            Self::AdaptiveLowActivityRent => "adaptive_rent_below_30pct_activity",
            Self::AdaptiveHighChangeLease => "adaptive_lease_at_70pct_change_events",
            Self::AdaptiveReuseConfidence => "adaptive_reuse_when_confidence_high",
            Self::AdaptiveCombined => "adaptive_low_activity_rent_high_change_lease_else_reuse",
            Self::PerfectInformationOracle => "perfect_information_family_oracle_non_deployable",
        }
    }

    fn deployable(self) -> bool {
        self != Self::PerfectInformationOracle
    }

    fn decision_overhead(self) -> f64 {
        match self {
            Self::FixedPurchase
            | Self::FixedLease
            | Self::FixedRent
            | Self::FixedRefurbish
            | Self::PerfectInformationOracle => 0.0,
            Self::AdaptiveLowActivityRent => 1_500.0,
            Self::AdaptiveHighChangeLease => 1_200.0,
            Self::AdaptiveReuseConfidence => 1_800.0,
            Self::AdaptiveCombined => 2_500.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct FamilyPolicyResult {
    family_id: String,
    policy_id: String,
    deployable_policy: bool,
    selected_strategy_id: String,
    selector_rule: String,
    source_family_signal_values: BTreeMap<String, f64>,
    signal_perturbation_case: String,
    lifecycle_npv_before_decision_overhead: f64,
    decision_and_measurement_overhead: f64,
    total_policy_npv_cost: f64,
    family_oracle_npv_cost: f64,
    regret_vs_family_oracle_npv_cost: f64,
    selected_strategy_is_family_cost_best: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PolicyScore {
    policy_id: String,
    deployable_policy: bool,
    mean_npv_cost_across_families: f64,
    worst_family_npv_cost: f64,
    best_family_npv_cost: f64,
    mean_regret_npv_cost: f64,
    maximum_regret_npv_cost: f64,
    mean_decision_overhead_npv_cost: f64,
    strategy_selection_counts: BTreeMap<String, usize>,
    families_where_policy_selects_family_cost_best_strategy_before_overhead: usize,
    family_count: usize,
    pareto_nondominated: bool,
    selection_rule_set: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct SensitivityCase {
    case_id: String,
    perturbed_signal: String,
    perturbation_description: String,
    policy_scores: Vec<PolicyScore>,
    policy_family_choices: Vec<FamilyPolicyResult>,
    choices_changed_from_base: usize,
    recommendation_changed_from_base: bool,
    evidence_class: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    source_catalog_sha256: String,
    source_lifecycle_report_sha256: String,
    evidence_class: String,
    cost_unit: String,
    discount_rate_per_year: f64,
    family_count: usize,
    family_ids: Vec<String>,
    strategy_ids: Vec<String>,
    policy_count: usize,
    policy_ids: Vec<String>,
    policy_definition_overhead_cost_units: BTreeMap<String, f64>,
    family_policy_results: Vec<FamilyPolicyResult>,
    policy_scores: Vec<PolicyScore>,
    pareto_frontier_policy_ids: Vec<String>,
    lowest_mean_npv_entry_including_oracle: String,
    best_deployable_policy_by_mean_npv: String,
    best_fixed_policy_by_mean_npv: String,
    best_adaptive_policy_by_mean_npv: String,
    value_of_adaptivity_vs_best_fixed_npv_cost_reduction: f64,
    value_of_adaptivity_percent_vs_best_fixed: Option<f64>,
    perfect_information_lower_bound_mean_npv_cost: f64,
    mean_perfect_information_gap_vs_best_adaptive_policy: f64,
    sensitivity_cases: Vec<SensitivityCase>,
    adaptive_policy_claim_ceiling: String,
    economic_recommendation_authorized: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

#[derive(Debug, Clone)]
struct Inputs {
    catalog: CatalogData,
    cost_by_family_strategy: BTreeMap<(String, String), f64>,
    best_by_family: BTreeMap<String, (String, f64)>,
    report_digest: String,
}

#[derive(Debug, Clone)]
struct ObservedSignals {
    active_fraction: f64,
    product_change_events_per_year: f64,
    reuse_success_fraction: f64,
    redeployability_fraction: f64,
    planning_horizon_years: u32,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("usage: ev_factory_adaptive_lifecycle_policies <catalog.json> <lifecycle-report.json>");
        process::exit(2);
    }
    let catalog_raw = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read catalog {}: {error}", args[0]);
            process::exit(2);
        }
    };
    let report_raw = match fs::read_to_string(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read lifecycle report {}: {error}", args[1]);
            process::exit(2);
        }
    };
    let catalog_value: Value = match serde_json::from_str(&catalog_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid lifecycle catalog JSON: {error}");
            process::exit(2);
        }
    };
    let report_value: Value = match serde_json::from_str(&report_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid lifecycle report JSON: {error}");
            process::exit(2);
        }
    };
    let inputs = match parse_inputs(&catalog_raw, &catalog_value, &report_raw, &report_value) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("adaptive policy inputs rejected: {error}");
            process::exit(2);
        }
    };
    let report = match evaluate(&inputs) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("adaptive policy evaluation failed: {error}");
            process::exit(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("cannot serialize adaptive lifecycle policy report: {error}");
            process::exit(2);
        }
    }
}

fn require_exact_keys(value: &Value, keys: &[&str], name: &str) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| format!("{name}_must_be_object"))?;
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = keys.iter().copied().collect();
    if actual != expected {
        return Err(format!("{name}_has_missing_or_unknown_fields"));
    }
    Ok(())
}

fn str_field<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str)
        .ok_or_else(|| format!("{key}_missing_or_not_string"))
}

fn f64_field(value: &Value, key: &str) -> Result<f64, String> {
    let number = value.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key}_missing_or_not_number"))?;
    if !number.is_finite() { return Err(format!("{key}_must_be_finite")); }
    Ok(number)
}

fn u64_field(value: &Value, key: &str) -> Result<u64, String> {
    value.get(key).and_then(Value::as_u64)
        .ok_or_else(|| format!("{key}_missing_or_not_unsigned_integer"))
}

fn bool_field(value: &Value, key: &str) -> Result<bool, String> {
    value.get(key).and_then(Value::as_bool)
        .ok_or_else(|| format!("{key}_missing_or_not_bool"))
}

fn parse_inputs(
    catalog_raw: &str,
    catalog: &Value,
    report_raw: &str,
    report: &Value,
) -> Result<Inputs, String> {
    let catalog_keys = [
        "schema_version", "benchmark_id", "evidence_class", "cost_unit", "time_unit",
        "discount_rate_per_year", "families", "strategies", "claim_ceiling",
    ];
    require_exact_keys(catalog, &catalog_keys, "catalog")?;
    if str_field(catalog, "schema_version")? != "0.1.0"
        || str_field(catalog, "benchmark_id")? != "EV-FACTORY-LIFECYCLE-001"
        || str_field(catalog, "evidence_class")? != "synthetic"
        || str_field(catalog, "time_unit")? != "year"
    {
        return Err("catalog_schema_benchmark_evidence_or_time_unit_unsupported".into());
    }
    let cost_unit = str_field(catalog, "cost_unit")?.to_string();
    if cost_unit.trim().is_empty() {
        return Err("cost_unit_must_be_nonempty".into());
    }
    let discount_rate = f64_field(catalog, "discount_rate_per_year")?;
    if !(0.0..=0.5).contains(&discount_rate) {
        return Err("discount_rate_out_of_supported_range".into());
    }

    let family_rows = catalog.get("families").and_then(Value::as_array)
        .ok_or_else(|| "families_missing_or_not_array".to_string())?;
    if family_rows.len() < 5 { return Err("at_least_five_product_families_required".into()); }
    let family_keys = [
        "family_id", "planning_horizon_years", "active_fraction",
        "product_change_events_per_year", "reuse_success_fraction",
        "redeployability_fraction", "salvage_realization_fraction",
    ];
    let mut family_signals = Vec::new();
    let mut family_ids = BTreeSet::new();
    for row in family_rows {
        require_exact_keys(row, &family_keys, "family_profile")?;
        let family_id = str_field(row, "family_id")?.to_string();
        if family_id.trim().is_empty() || !family_ids.insert(family_id.clone()) {
            return Err("family_ids_must_be_nonempty_and_unique".into());
        }
        let horizon = u64_field(row, "planning_horizon_years")?;
        if horizon == 0 || horizon > 30 {
            return Err("planning_horizon_must_be_between_one_and_thirty".into());
        }
        let active = f64_field(row, "active_fraction")?;
        let changes = f64_field(row, "product_change_events_per_year")?;
        let reuse = f64_field(row, "reuse_success_fraction")?;
        let redeploy = f64_field(row, "redeployability_fraction")?;
        let salvage = f64_field(row, "salvage_realization_fraction")?;
        for (label, fraction) in [
            ("active_fraction", active),
            ("reuse_success_fraction", reuse),
            ("redeployability_fraction", redeploy),
            ("salvage_realization_fraction", salvage),
        ] {
            if !(0.0..=1.0).contains(&fraction) {
                return Err(format!("{label}_outside_zero_to_one"));
            }
        }
        if changes < 0.0 { return Err("product_change_events_per_year_must_be_nonnegative".into()); }
        family_signals.push(FamilySignal {
            family_id,
            planning_horizon_years: horizon as u32,
            active_fraction: active,
            product_change_events_per_year: changes,
            reuse_success_fraction: reuse,
            redeployability_fraction: redeploy,
            salvage_realization_fraction: salvage,
        });
    }
    let strategy_rows = catalog.get("strategies").and_then(Value::as_array)
        .ok_or_else(|| "strategies_missing_or_not_array".to_string())?;
    let strategy_keys = [
        "strategy_id", "kind", "owns_asset", "acquisition_cost", "refurbishment_cost",
        "installation_cost", "annual_contract_cost", "contract_payment_basis",
        "annual_maintenance_cost", "annual_idle_carry_cost", "retooling_cost_per_product_change",
        "fallback_replacement_cost_if_reuse_fails", "terminal_salvage_value",
        "terminal_disposal_cost", "note",
    ];
    let mut strategies = Vec::new();
    let mut ids = BTreeSet::new();
    let mut kinds = BTreeSet::new();
    for row in strategy_rows {
        require_exact_keys(row, &strategy_keys, "strategy")?;
        let id = str_field(row, "strategy_id")?.to_string();
        let kind = str_field(row, "kind")?.to_string();
        if id.trim().is_empty() || !ids.insert(id.clone()) || !kinds.insert(kind.clone()) {
            return Err("strategy_ids_and_kinds_must_be_unique".into());
        }
        strategies.push(StrategyDefinition { strategy_id: id, kind });
    }
    if kinds != BTreeSet::from([
        "purchase_new".to_string(),
        "lease".to_string(),
        "short_term_rent".to_string(),
        "refurbish_redeploy".to_string(),
    ]) {
        return Err("required_four_strategy_kinds_missing".into());
    }

    let exact_report_keys = [
        "schema_version", "benchmark_id", "source_catalog_sha256", "source_evidence_class",
        "cost_unit", "time_unit", "discount_rate_per_year", "family_count", "family_ids",
        "strategy_count", "strategy_ids", "family_strategy_results", "strategy_scores",
        "pareto_frontier_strategy_ids", "purchase_vs_lease_break_even", "sensitivity_results",
        "family_aggregation_note", "economic_recommendation_authorized",
        "production_release_authorized", "claim_ceiling",
    ];
    require_exact_keys(report, &exact_report_keys, "lifecycle_report")?;
    let catalog_sha = format!("{:x}", Sha256::digest(catalog_raw.as_bytes()));
    let report_sha = format!("{:x}", Sha256::digest(report_raw.as_bytes()));
    if str_field(report, "schema_version")? != "0.1.0"
        || str_field(report, "benchmark_id")? != "EV-FACTORY-LIFECYCLE-001"
        || str_field(report, "source_evidence_class")? != "synthetic"
        || str_field(report, "source_catalog_sha256")? != catalog_sha
        || str_field(report, "cost_unit")? != cost_unit
        || (f64_field(report, "discount_rate_per_year")? - discount_rate).abs() > 1e-12
        || bool_field(report, "economic_recommendation_authorized")?
        || bool_field(report, "production_release_authorized")?
    {
        return Err("lifecycle_report_does_not_match_exact_synthetic_catalog".into());
    }
    let report_family_ids: BTreeSet<String> = report.get("family_ids").and_then(Value::as_array)
        .ok_or_else(|| "report_family_ids_missing".to_string())?
        .iter().map(|value| value.as_str().map(str::to_string).ok_or_else(|| "report_family_id_not_string".to_string()))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if report_family_ids != family_ids { return Err("report_family_ids_do_not_match_catalog".into()); }
    let report_strategy_ids: BTreeSet<String> = report.get("strategy_ids").and_then(Value::as_array)
        .ok_or_else(|| "report_strategy_ids_missing".to_string())?
        .iter().map(|value| value.as_str().map(str::to_string).ok_or_else(|| "report_strategy_id_not_string".to_string()))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if report_strategy_ids != ids { return Err("report_strategy_ids_do_not_match_catalog".into()); }

    let rows = report.get("family_strategy_results").and_then(Value::as_array)
        .ok_or_else(|| "family_strategy_results_missing".to_string())?;
    if rows.len() != family_signals.len() * strategies.len() {
        return Err("family_strategy_result_count_mismatch".into());
    }
    let mut cost_by_family_strategy = BTreeMap::new();
    let mut best_by_family = BTreeMap::new();
    for row in rows {
        let family_id = str_field(row, "family_id")?.to_string();
        let strategy_id = str_field(row, "strategy_id")?.to_string();
        let total_npv = f64_field(row, "total_npv_cost")?;
        let regret = f64_field(row, "regret_vs_family_best_npv_cost")?;
        if !family_ids.contains(&family_id) || !ids.contains(&strategy_id) || regret < -1e-9 {
            return Err("family_strategy_cost_row_has_unknown_id_or_invalid_value".into());
        }
        if cost_by_family_strategy.insert((family_id.clone(), strategy_id.clone()), total_npv).is_some() {
            return Err("duplicate_family_strategy_cost_row".into());
        }
        let entry: &mut (String, f64) = best_by_family.entry(family_id.clone()).or_insert((strategy_id.clone(), total_npv));
        if total_npv < entry.1 { *entry = (strategy_id, total_npv); }
    }
    if cost_by_family_strategy.len() != rows.len() || best_by_family.len() != family_signals.len() {
        return Err("incomplete_family_strategy_cost_matrix".into());
    }
    Ok(Inputs {
        catalog: CatalogData {
            source_sha256: catalog_sha,
            cost_unit,
            discount_rate_per_year: discount_rate,
            family_signals,
            strategies,
        },
        cost_by_family_strategy,
        best_by_family,
        report_digest: report_sha,
    })
}

fn strategy_id_by_kind<'a>(catalog: &'a CatalogData, kind: &str) -> Result<&'a str, String> {
    catalog.strategies.iter().find(|strategy| strategy.kind == kind)
        .map(|strategy| strategy.strategy_id.as_str())
        .ok_or_else(|| format!("strategy_kind_{kind}_missing"))
}

fn selected_strategy(policy: PolicyKind, signal: &ObservedSignals, inputs: &Inputs) -> Result<(String, String), String> {
    let strategy = match policy {
        PolicyKind::FixedPurchase => strategy_id_by_kind(&inputs.catalog, "purchase_new")?,
        PolicyKind::FixedLease => strategy_id_by_kind(&inputs.catalog, "lease")?,
        PolicyKind::FixedRent => strategy_id_by_kind(&inputs.catalog, "short_term_rent")?,
        PolicyKind::FixedRefurbish => strategy_id_by_kind(&inputs.catalog, "refurbish_redeploy")?,
        PolicyKind::AdaptiveLowActivityRent => {
            if signal.active_fraction < LOW_ACTIVITY_THRESHOLD {
                strategy_id_by_kind(&inputs.catalog, "short_term_rent")?
            } else {
                strategy_id_by_kind(&inputs.catalog, "refurbish_redeploy")?
            }
        }
        PolicyKind::AdaptiveHighChangeLease => {
            if signal.product_change_events_per_year >= HIGH_CHANGE_THRESHOLD {
                strategy_id_by_kind(&inputs.catalog, "lease")?
            } else {
                strategy_id_by_kind(&inputs.catalog, "refurbish_redeploy")?
            }
        }
        PolicyKind::AdaptiveReuseConfidence => {
            if signal.reuse_success_fraction >= REUSE_SUCCESS_THRESHOLD
                && signal.redeployability_fraction >= REDEPLOYABILITY_THRESHOLD
            {
                strategy_id_by_kind(&inputs.catalog, "refurbish_redeploy")?
            } else {
                strategy_id_by_kind(&inputs.catalog, "lease")?
            }
        }
        PolicyKind::AdaptiveCombined => {
            if signal.active_fraction < LOW_ACTIVITY_THRESHOLD {
                strategy_id_by_kind(&inputs.catalog, "short_term_rent")?
            } else if signal.product_change_events_per_year >= HIGH_CHANGE_THRESHOLD {
                strategy_id_by_kind(&inputs.catalog, "lease")?
            } else if signal.reuse_success_fraction >= REUSE_SUCCESS_THRESHOLD
                && signal.redeployability_fraction >= REDEPLOYABILITY_THRESHOLD
            {
                strategy_id_by_kind(&inputs.catalog, "refurbish_redeploy")?
            } else {
                strategy_id_by_kind(&inputs.catalog, "lease")?
            }
        }
        PolicyKind::PerfectInformationOracle => {
            return Err("perfect_information_oracle_requires_family_specific_costs".into());
        }
    }.to_string();
    Ok((strategy.clone(), selector_rule(policy, signal)))
}

fn selector_rule(policy: PolicyKind, signal: &ObservedSignals) -> String {
    match policy {
        PolicyKind::FixedPurchase => "fixed: purchase_new for every family".into(),
        PolicyKind::FixedLease => "fixed: lease for every family".into(),
        PolicyKind::FixedRent => "fixed: short_term_rent for every family".into(),
        PolicyKind::FixedRefurbish => "fixed: refurbish_redeploy for every family".into(),
        PolicyKind::AdaptiveLowActivityRent => format!("rent if observed active_fraction < {LOW_ACTIVITY_THRESHOLD:.2}; else refurbish_redeploy (observed {:.2})", signal.active_fraction),
        PolicyKind::AdaptiveHighChangeLease => format!("lease if observed product_change_events_per_year >= {HIGH_CHANGE_THRESHOLD:.2}; else refurbish_redeploy (observed {:.2})", signal.product_change_events_per_year),
        PolicyKind::AdaptiveReuseConfidence => format!("refurbish when observed reuse_success >= {REUSE_SUCCESS_THRESHOLD:.2} and redeployability >= {REDEPLOYABILITY_THRESHOLD:.2}; otherwise lease (reuse {:.2}, redeploy {:.2})", signal.reuse_success_fraction, signal.redeployability_fraction),
        PolicyKind::AdaptiveCombined => format!("rent if active_fraction < {LOW_ACTIVITY_THRESHOLD:.2}; else lease if product changes >= {HIGH_CHANGE_THRESHOLD:.2}; else refurbish if reuse success and redeployability thresholds pass (active {:.2}, changes {:.2}, reuse {:.2}, redeploy {:.2})",signal.active_fraction, signal.product_change_events_per_year, signal.reuse_success_fraction, signal.redeployability_fraction),
        PolicyKind::PerfectInformationOracle => "non-deployable lower-bound oracle: selects cheapest strategy using outcome costs".into(),
    }
}

fn policy_rule_description(policy: PolicyKind) -> String {
    match policy {
        PolicyKind::FixedPurchase => "fixed: purchase_new for every family".into(),
        PolicyKind::FixedLease => "fixed: lease for every family".into(),
        PolicyKind::FixedRent => "fixed: short_term_rent for every family".into(),
        PolicyKind::FixedRefurbish => "fixed: refurbish_redeploy for every family".into(),
        PolicyKind::AdaptiveLowActivityRent => format!(
            "adaptive: rent when observed active_fraction < {LOW_ACTIVITY_THRESHOLD:.2}, else refurbish_redeploy"
        ),
        PolicyKind::AdaptiveHighChangeLease => format!(
            "adaptive: lease when observed product_change_events_per_year >= {HIGH_CHANGE_THRESHOLD:.2}, else refurbish_redeploy"
        ),
        PolicyKind::AdaptiveReuseConfidence => format!(
            "adaptive: refurbish when observed reuse_success >= {REUSE_SUCCESS_THRESHOLD:.2} and redeployability >= {REDEPLOYABILITY_THRESHOLD:.2}, else lease"
        ),
        PolicyKind::AdaptiveCombined => format!(
            "adaptive: rent when active_fraction < {LOW_ACTIVITY_THRESHOLD:.2}; else lease when product changes >= {HIGH_CHANGE_THRESHOLD:.2}; else refurbish when reuse/redeployability thresholds pass; fallback lease"
        ),
        PolicyKind::PerfectInformationOracle => {
            "non-deployable lower bound: choose family cost oracle; not an operational policy".into()
        }
    }
}

fn make_observed(profile: &FamilySignal, case_id: &str) -> ObservedSignals {
    let mut observed = ObservedSignals {
        active_fraction: profile.active_fraction,
        product_change_events_per_year: profile.product_change_events_per_year,
        reuse_success_fraction: profile.reuse_success_fraction,
        redeployability_fraction: profile.redeployability_fraction,
        planning_horizon_years: profile.planning_horizon_years,
    };
    match case_id {
        "activity_minus_0_05" => observed.active_fraction = (observed.active_fraction - 0.05).max(0.0),
        "change_frequency_plus_0_15" => observed.product_change_events_per_year += 0.15,
        "reuse_signal_minus_0_05" => {
            observed.reuse_success_fraction = (observed.reuse_success_fraction - 0.05).max(0.0);
            observed.redeployability_fraction = (observed.redeployability_fraction - 0.05).max(0.0);
        }
        _ => {}
    }
    observed
}

fn score_policy(policy: PolicyKind, choices: &[FamilyPolicyResult], family_count: usize) -> PolicyScore {
    let costs: Vec<f64> = choices.iter().map(|row| row.total_policy_npv_cost).collect();
    let regrets: Vec<f64> = choices.iter().map(|row| row.regret_vs_family_oracle_npv_cost).collect();
    let best_count = choices.iter().filter(|row| row.selected_strategy_is_family_cost_best).count();
    let mut counts = BTreeMap::new();
    for row in choices { *counts.entry(row.selected_strategy_id.clone()).or_insert(0usize) += 1; }
    PolicyScore {
        policy_id: policy.id().to_string(),
        deployable_policy: policy.deployable(),
        mean_npv_cost_across_families: costs.iter().sum::<f64>() / costs.len() as f64,
        worst_family_npv_cost: costs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        best_family_npv_cost: costs.iter().copied().fold(f64::INFINITY, f64::min),
        mean_regret_npv_cost: regrets.iter().sum::<f64>() / regrets.len() as f64,
        maximum_regret_npv_cost: regrets.iter().copied().fold(0.0, f64::max),
        mean_decision_overhead_npv_cost: choices.iter().map(|row| row.decision_and_measurement_overhead).sum::<f64>() / costs.len() as f64,
        strategy_selection_counts: counts,
        families_where_policy_selects_family_cost_best_strategy_before_overhead: best_count,
        family_count,
        pareto_nondominated: false,
        selection_rule_set: vec![policy_rule_description(policy)],
    }
}

fn evaluate_policy(
    policy: PolicyKind,
    inputs: &Inputs,
    signal_case: &str,
) -> Result<Vec<FamilyPolicyResult>, String> {
    let mut choices = Vec::new();
    for family in &inputs.catalog.family_signals {
        let observed = make_observed(family, signal_case);
        let (strategy_id, rule) = if policy == PolicyKind::PerfectInformationOracle {
            (inputs.best_by_family[&family.family_id].0.clone(), selector_rule(policy, &observed))
        } else {
            selected_strategy(policy, &observed, inputs)?
        };
        let raw_cost = *inputs.cost_by_family_strategy
            .get(&(family.family_id.clone(), strategy_id.clone()))
            .ok_or_else(|| "policy_selected_strategy_cost_not_in_lifecycle_report".to_string())?;
        let oracle = inputs.best_by_family[&family.family_id].1;
        let overhead = policy.decision_overhead();
        let total = raw_cost + overhead;
        choices.push(FamilyPolicyResult {
            family_id: family.family_id.clone(),
            policy_id: policy.id().to_string(),
            deployable_policy: policy.deployable(),
            selected_strategy_id: strategy_id.clone(),
            selector_rule: rule,
            source_family_signal_values: BTreeMap::from([
                ("active_fraction".into(), observed.active_fraction),
                ("product_change_events_per_year".into(), observed.product_change_events_per_year),
                ("reuse_success_fraction".into(), observed.reuse_success_fraction),
                ("redeployability_fraction".into(), observed.redeployability_fraction),
                ("planning_horizon_years".into(), observed.planning_horizon_years as f64),
            ]),
            signal_perturbation_case: signal_case.to_string(),
            lifecycle_npv_before_decision_overhead: raw_cost,
            decision_and_measurement_overhead: overhead,
            total_policy_npv_cost: total,
            family_oracle_npv_cost: oracle,
            regret_vs_family_oracle_npv_cost: (total - oracle).max(0.0),
            selected_strategy_is_family_cost_best: strategy_id == inputs.best_by_family[&family.family_id].0,
        });
    }
    Ok(choices)
}

fn evaluate(inputs: &Inputs) -> Result<Report, String> {
    let policies = PolicyKind::ALL.to_vec();
    let mut family_policy_results = Vec::new();
    let mut policy_scores = Vec::new();
    for policy in &policies {
        let choices = evaluate_policy(*policy, inputs, "base")?;
        let mut score = score_policy(*policy, &choices, inputs.catalog.family_signals.len());
        score.selection_rule_set = vec![policy_rule_description(*policy)];
        policy_scores.push(score);
        family_policy_results.extend(choices);
    }
    let mut frontier_ids = Vec::new();
    for i in 0..policy_scores.len() {
        let dominated = (0..policy_scores.len()).any(|j| {
            if i == j { return false; }
            let other = &policy_scores[j];
            let this = &policy_scores[i];
            let no_worse = other.mean_npv_cost_across_families <= this.mean_npv_cost_across_families
                && other.worst_family_npv_cost <= this.worst_family_npv_cost
                && other.maximum_regret_npv_cost <= this.maximum_regret_npv_cost;
            let strictly_better = other.mean_npv_cost_across_families < this.mean_npv_cost_across_families
                || other.worst_family_npv_cost < this.worst_family_npv_cost
                || other.maximum_regret_npv_cost < this.maximum_regret_npv_cost;
            no_worse && strictly_better
        });
        policy_scores[i].pareto_nondominated = !dominated;
        if !dominated { frontier_ids.push(policy_scores[i].policy_id.clone()); }
    }
    let deployable_scores: Vec<&PolicyScore> = policy_scores.iter().filter(|score| score.deployable_policy).collect();
    let best_deployable = deployable_scores.iter().min_by(|a,b| a.mean_npv_cost_across_families.total_cmp(&b.mean_npv_cost_across_families))
        .ok_or_else(|| "no_deployable_policy_scores".to_string())?;
    let fixed_scores: Vec<&PolicyScore> = deployable_scores.iter().copied().filter(|score| score.policy_id.starts_with("fixed_")).collect();
    let best_fixed = fixed_scores.iter().min_by(|a,b| a.mean_npv_cost_across_families.total_cmp(&b.mean_npv_cost_across_families))
        .ok_or_else(|| "no_fixed_policy_scores".to_string())?;
    let lowest_mean_entry = policy_scores.iter().min_by(|a,b| a.mean_npv_cost_across_families.total_cmp(&b.mean_npv_cost_across_families))
        .ok_or_else(|| "no_policy_scores".to_string())?;
    let best_adaptive = policy_scores.iter()
        .filter(|score| score.policy_id.starts_with("adaptive_"))
        .min_by(|a,b| a.mean_npv_cost_across_families.total_cmp(&b.mean_npv_cost_across_families))
        .ok_or_else(|| "no_adaptive_policy_scores".to_string())?;
    let oracle = policy_scores.iter().find(|score| score.policy_id == PolicyKind::PerfectInformationOracle.id()).expect("oracle score exists");

    let mut sensitivity_cases = Vec::new();
    for (case_id, signal) in [
        ("activity_minus_0_05", "active_fraction"),
        ("change_frequency_plus_0_15", "product_change_events_per_year"),
        ("reuse_signal_minus_0_05", "reuse_success_and_redeployability"),
    ] {
        let mut choices_all = Vec::new();
        let mut scores = Vec::new();
        for policy in &policies {
            let choices = evaluate_policy(*policy, inputs, case_id)?;
            scores.push(score_policy(*policy, &choices, inputs.catalog.family_signals.len()));
            choices_all.extend(choices);
        }
        let (base_best, noisy_best) = (
            best_deployable.policy_id.as_str(),
            scores.iter().filter(|score| score.deployable_policy)
                .min_by(|a,b| a.mean_npv_cost_across_families.total_cmp(&b.mean_npv_cost_across_families))
                .map(|score| score.policy_id.as_str()).unwrap_or("none"),
        );
        let base_lookup: BTreeMap<(String, String), String> = family_policy_results.iter()
            .map(|row| ((row.policy_id.clone(), row.family_id.clone()), row.selected_strategy_id.clone())).collect();
        let changed = choices_all.iter().filter(|row| {
            base_lookup.get(&(row.policy_id.clone(), row.family_id.clone()))
                .is_some_and(|base| base != &row.selected_strategy_id)
        }).count();
        sensitivity_cases.push(SensitivityCase {
            case_id: case_id.into(),
            perturbed_signal: signal.into(),
            perturbation_description: case_id.replace('_', " "),
            policy_scores: scores,
            policy_family_choices: choices_all,
            choices_changed_from_base: changed,
            recommendation_changed_from_base: base_best != noisy_best,
            evidence_class: "synthetic".into(),
        });
    }
    Ok(Report {
        schema_version: "0.1.0".into(),
        benchmark_id: BENCHMARK_ID.into(),
        source_catalog_sha256: inputs.catalog.source_sha256.clone(),
        source_lifecycle_report_sha256: inputs.report_digest.clone(),
        evidence_class: "synthetic".into(),
        cost_unit: inputs.catalog.cost_unit.clone(),
        discount_rate_per_year: inputs.catalog.discount_rate_per_year,
        family_count: inputs.catalog.family_signals.len(),
        family_ids: inputs.catalog.family_signals.iter().map(|f| f.family_id.clone()).collect(),
        strategy_ids: inputs.catalog.strategies.iter().map(|s| s.strategy_id.clone()).collect(),
        policy_count: policies.len(),
        policy_ids: policies.iter().map(|policy| policy.id().to_string()).collect(),
        policy_definition_overhead_cost_units: policies.iter().map(|policy| (policy.id().to_string(), policy.decision_overhead())).collect(),
        family_policy_results,
        policy_scores,
        pareto_frontier_policy_ids: frontier_ids,
        lowest_mean_npv_entry_including_oracle: lowest_mean_entry.policy_id.clone(),
        best_deployable_policy_by_mean_npv: best_deployable.policy_id.clone(),
        best_fixed_policy_by_mean_npv: best_fixed.policy_id.clone(),
        best_adaptive_policy_by_mean_npv: best_adaptive.policy_id.clone(),
        value_of_adaptivity_vs_best_fixed_npv_cost_reduction: best_fixed.mean_npv_cost_across_families - best_adaptive.mean_npv_cost_across_families,
        value_of_adaptivity_percent_vs_best_fixed: if best_fixed.mean_npv_cost_across_families.abs() > 1e-9 {
            Some((best_fixed.mean_npv_cost_across_families - best_adaptive.mean_npv_cost_across_families) / best_fixed.mean_npv_cost_across_families.abs() * 100.0)
        } else { None },
        perfect_information_lower_bound_mean_npv_cost: oracle.mean_npv_cost_across_families,
        mean_perfect_information_gap_vs_best_adaptive_policy: (best_deployable.mean_npv_cost_across_families - oracle.mean_npv_cost_across_families).max(0.0),
        sensitivity_cases,
        adaptive_policy_claim_ceiling: "Adaptive rules use only family profile signals (activity, product-change rate, reuse success and redeployability), never family IDs or per-family cost/oracle labels. Rules and overheads are declared in code and must be preregistered before measured-data use.".into(),
        economic_recommendation_authorized: false,
        production_release_authorized: false,
        claim_ceiling: CLAIM_CEILING.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_inputs() -> Inputs {
        let catalog_raw = include_str!("../../fixtures/ev_factory_lifecycle_v0_1.json");
        let catalog_value: Value = serde_json::from_str(catalog_raw).unwrap();
        let digest = format!("{:x}", Sha256::digest(catalog_raw.as_bytes()));
        let families = catalog_value["families"].as_array().unwrap();
        let strategies = catalog_value["strategies"].as_array().unwrap();
        let family_ids: Vec<String> = families
            .iter()
            .map(|item| item["family_id"].as_str().unwrap().to_string())
            .collect();
        let strategy_ids: Vec<String> = strategies
            .iter()
            .map(|item| item["strategy_id"].as_str().unwrap().to_string())
            .collect();
        let mut rows = Vec::new();
        for (family_index, family_id) in family_ids.iter().enumerate() {
            let mut costs = Vec::new();
            for (strategy_index, strategy_id) in strategy_ids.iter().enumerate() {
                // Synthetic rows exercise the selection contract; hosted CI supplies
                // the report generated by the lifecycle calculator itself.
                let mut cost = 100_000.0 + family_index as f64 * 1_000.0;
                cost += match strategies[strategy_index]["kind"].as_str().unwrap() {
                    "purchase_new" => 45_000.0,
                    "lease" => 65_000.0,
                    "short_term_rent" if family_id == "product-retirement-redeploy" => -30_000.0,
                    "short_term_rent" => 90_000.0,
                    "refurbish_redeploy" if family_id == "product-retirement-redeploy" => 20_000.0,
                    "refurbish_redeploy" => 0.0,
                    _ => unreachable!(),
                };
                costs.push((strategy_id.clone(), cost));
            }
            let family_best = costs.iter().map(|(_, cost)| *cost).fold(f64::INFINITY, f64::min);
            for (strategy_id, cost) in costs {
                rows.push(serde_json::json!({
                    "family_id": family_id,
                    "strategy_id": strategy_id,
                    "total_npv_cost": cost,
                    "regret_vs_family_best_npv_cost": cost - family_best
                }));
            }
        }
        let report_value = serde_json::json!({
            "schema_version": "0.1.0",
            "benchmark_id": "EV-FACTORY-LIFECYCLE-001",
            "source_catalog_sha256": digest,
            "source_evidence_class": "synthetic",
            "cost_unit": catalog_value["cost_unit"],
            "time_unit": "year",
            "discount_rate_per_year": catalog_value["discount_rate_per_year"],
            "family_count": family_ids.len(),
            "family_ids": family_ids,
            "strategy_count": strategy_ids.len(),
            "strategy_ids": strategy_ids,
            "family_strategy_results": rows,
            "strategy_scores": [],
            "pareto_frontier_strategy_ids": [],
            "purchase_vs_lease_break_even": [],
            "sensitivity_results": [],
            "family_aggregation_note": "synthetic unit fixture",
            "economic_recommendation_authorized": false,
            "production_release_authorized": false,
            "claim_ceiling": "synthetic"
        });
        let report_raw = serde_json::to_string(&report_value).unwrap();
        parse_inputs(catalog_raw, &catalog_value, &report_raw, &report_value).unwrap()
    }

    #[test]
    fn adaptive_selection_uses_signal_not_family_identity_or_oracle_cost() {
        let inputs = fixture_inputs();
        let low_profile = inputs.catalog.family_signals.iter()
            .find(|family| family.active_fraction < LOW_ACTIVITY_THRESHOLD)
            .expect("fixture includes a low-activity scenario");
        let low = make_observed(low_profile, "base");
        let (selected, _) = selected_strategy(PolicyKind::AdaptiveLowActivityRent, &low, &inputs).unwrap();
        assert_eq!(selected, strategy_id_by_kind(&inputs.catalog, "short_term_rent").unwrap());
        let high_profile = inputs.catalog.family_signals.iter()
            .find(|family| family.active_fraction >= LOW_ACTIVITY_THRESHOLD)
            .expect("fixture includes a higher-activity scenario");
        let high = make_observed(high_profile, "base");
        let (selected, _) = selected_strategy(PolicyKind::AdaptiveLowActivityRent, &high, &inputs).unwrap();
        assert_eq!(selected, strategy_id_by_kind(&inputs.catalog, "refurbish_redeploy").unwrap());
    }

    #[test]
    fn adaptive_policy_charges_overhead_and_oracle_is_non_deployable() {
        let inputs = fixture_inputs();
        let report = evaluate(&inputs).unwrap();
        let adapt = report.policy_scores.iter().find(|score| score.policy_id == PolicyKind::AdaptiveLowActivityRent.id()).unwrap();
        let fixed = report.policy_scores.iter().find(|score| score.policy_id == PolicyKind::FixedRefurbish.id()).unwrap();
        assert_eq!(adapt.mean_decision_overhead_npv_cost, 1500.0);
        assert!(!report.policy_scores.iter().find(|score| score.policy_id == PolicyKind::PerfectInformationOracle.id()).unwrap().deployable_policy);
        assert!(!report.economic_recommendation_authorized);
        assert!(!report.production_release_authorized);
        let raw_mean = report.family_policy_results.iter()
            .filter(|row| row.policy_id == PolicyKind::AdaptiveLowActivityRent.id())
            .map(|row| row.lifecycle_npv_before_decision_overhead)
            .sum::<f64>() / 5.0;
        assert!((adapt.mean_npv_cost_across_families - (raw_mean + 1500.0)).abs() < 1e-9);
        assert_eq!(fixed.mean_decision_overhead_npv_cost, 0.0);
        assert_eq!(report.best_adaptive_policy_by_mean_npv, adapt.policy_id);
    }

    #[test]
    fn each_policy_has_one_decision_per_family_and_sensitivity_is_explicit() {
        let report = evaluate(&fixture_inputs()).unwrap();
        assert_eq!(report.family_policy_results.len(), 45);
        assert_eq!(report.sensitivity_cases.len(), 3);
        assert!(report.sensitivity_cases.iter().all(|case| case.policy_scores.len() == 9));
        assert_eq!(report.policy_scores.len(), 9);
    }
}

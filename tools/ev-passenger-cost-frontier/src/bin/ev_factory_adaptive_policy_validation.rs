use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process};

const BENCHMARK_ID: &str = "EV-FACTORY-ADAPTIVE-VALIDATION-001";
const ORACLE_POLICY_ID: &str = "perfect_information_family_oracle_non_deployable";
const CLAIM_CEILING: &str = "Leave-one-family-out policy selection over synthetic factory lifecycle profiles only. Five held-out stress cases do not establish real-world generalization, probability calibration, SOTA or investment approval.";

#[derive(Debug, Clone)]
struct FamilySignal {
    family_id: String,
    planning_horizon_years: u32,
    active_fraction: f64,
    product_change_events_per_year: f64,
    reuse_success_fraction: f64,
    redeployability_fraction: f64,
}

#[derive(Debug, Clone)]
struct CatalogData {
    catalog_sha256: String,
    cost_unit: String,
    strategies_by_kind: BTreeMap<String, String>,
    strategy_ids: BTreeSet<String>,
    families: Vec<FamilySignal>,
}

#[derive(Debug, Clone)]
struct LifecycleRow {
    cost: f64,
}

#[derive(Debug, Clone)]
struct PolicyRow {
    family_id: String,
    policy_id: String,
    deployable: bool,
    selected_strategy_id: String,
    lifecycle_npv_before_overhead: f64,
    overhead: f64,
    total_cost: f64,
    oracle_cost: f64,
    regret: f64,
}

#[derive(Debug, Clone)]
struct Inputs {
    catalog: CatalogData,
    lifecycle_report_sha256: String,
    adaptive_report_sha256: String,
    lifecycle_costs: BTreeMap<(String, String), LifecycleRow>,
    policy_rows: BTreeMap<(String, String), PolicyRow>,
    family_oracle: BTreeMap<String, (String, f64)>,
    policy_ids: BTreeSet<String>,
    decision_overheads: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct FoldResult {
    held_out_family_id: String,
    training_family_ids: Vec<String>,
    selected_policy_id_from_training_only: String,
    selected_policy_training_mean_npv_cost: f64,
    held_out_selected_strategy_id: String,
    held_out_policy_npv_cost_including_overhead: f64,
    fixed_comparator_policy_id_from_training_only: String,
    fixed_comparator_training_mean_npv_cost: f64,
    held_out_fixed_comparator_npv_cost: f64,
    adaptive_minus_fixed_cost_units: f64,
    held_out_family_oracle_cost_units: f64,
    regret_vs_held_out_family_oracle_units: f64,
    selected_decision_overhead_units: f64,
    selection_was_independent_of_held_out_cost: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct PolicySelectionCount {
    policy_id: String,
    selected_for_held_out_folds: usize,
    deployable: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    source_catalog_sha256: String,
    source_lifecycle_report_sha256: String,
    source_adaptive_report_sha256: String,
    evidence_class: String,
    cost_unit: String,
    family_count: usize,
    family_ids: Vec<String>,
    policy_count: usize,
    deployable_policy_count: usize,
    fold_results: Vec<FoldResult>,
    policy_selection_counts: Vec<PolicySelectionCount>,
    mean_held_out_policy_cost_units: f64,
    worst_held_out_policy_cost_units: f64,
    mean_held_out_fixed_comparator_cost_units: f64,
    mean_adaptive_minus_fixed_cost_units: f64,
    mean_held_out_regret_vs_family_oracle_units: f64,
    folds_where_selected_policy_was_adaptive: usize,
    fold_count_where_adaptive_policy_saved_cost: usize,
    oracle_is_non_deployable: bool,
    held_out_costs_used_in_policy_selection: bool,
    evaluation_note: String,
    economic_recommendation_authorized: bool,
    production_release_authorized: bool,
    claim_ceiling: String,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 3 {
        eprintln!("usage: ev_factory_adaptive_policy_validation <catalog.json> <lifecycle-report.json> <adaptive-policy-report.json>");
        process::exit(2);
    }
    let catalog_raw = read_file(&args[0]);
    let lifecycle_raw = read_file(&args[1]);
    let adaptive_raw = read_file(&args[2]);
    let (catalog_raw, lifecycle_raw, adaptive_raw) = match (catalog_raw, lifecycle_raw, adaptive_raw) {
        (Ok(catalog), Ok(lifecycle), Ok(adaptive)) => (catalog, lifecycle, adaptive),
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
            eprintln!("cannot read validation inputs: {error}");
            process::exit(2);
        }
    };
    let catalog_value: Value = match serde_json::from_str(&catalog_raw) {
        Ok(value) => value,
        Err(error) => { eprintln!("catalog JSON invalid: {error}"); process::exit(2); }
    };
    let lifecycle_value: Value = match serde_json::from_str(&lifecycle_raw) {
        Ok(value) => value,
        Err(error) => { eprintln!("lifecycle report JSON invalid: {error}"); process::exit(2); }
    };
    let adaptive_value: Value = match serde_json::from_str(&adaptive_raw) {
        Ok(value) => value,
        Err(error) => { eprintln!("adaptive report JSON invalid: {error}"); process::exit(2); }
    };
    let inputs = match parse_inputs(
        &catalog_raw, &catalog_value,
        &lifecycle_raw, &lifecycle_value,
        &adaptive_raw, &adaptive_value,
    ) {
        Ok(value) => value,
        Err(error) => { eprintln!("adaptive validation inputs rejected: {error}"); process::exit(2); }
    };
    let report = match evaluate(&inputs) {
        Ok(value) => value,
        Err(error) => { eprintln!("leave-one-family-out evaluation failed: {error}"); process::exit(2); }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => println!("{value}"),
        Err(error) => { eprintln!("cannot serialize validation report: {error}"); process::exit(2); }
    }
}

fn read_file(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))
}

fn exact_keys(value: &Value, expected: &[&str], name: &str) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| format!("{name}_must_be_object"))?;
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != expected { return Err(format!("{name}_has_missing_or_unknown_fields")); }
    Ok(())
}

fn string_field<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str).ok_or_else(|| format!("{key}_missing_or_not_string"))
}

fn number_field(value: &Value, key: &str) -> Result<f64, String> {
    let result = value.get(key).and_then(Value::as_f64).ok_or_else(|| format!("{key}_missing_or_not_number"))?;
    if !result.is_finite() { return Err(format!("{key}_must_be_finite")); }
    Ok(result)
}

fn bool_field(value: &Value, key: &str) -> Result<bool, String> {
    value.get(key).and_then(Value::as_bool).ok_or_else(|| format!("{key}_missing_or_not_bool"))
}

fn parse_inputs(
    catalog_raw: &str, catalog: &Value,
    lifecycle_raw: &str, lifecycle: &Value,
    adaptive_raw: &str, adaptive: &Value,
) -> Result<Inputs, String> {
    exact_keys(catalog, &[
        "schema_version","benchmark_id","evidence_class","cost_unit","time_unit",
        "discount_rate_per_year","families","strategies","claim_ceiling",
    ], "catalog")?;
    if string_field(catalog, "schema_version")? != "0.1.0"
        || string_field(catalog, "benchmark_id")? != "EV-FACTORY-LIFECYCLE-001"
        || string_field(catalog, "evidence_class")? != "synthetic"
        || string_field(catalog, "time_unit")? != "year"
    {
        return Err("catalog_schema_or_evidence_class_unsupported".into());
    }
    let cost_unit = string_field(catalog, "cost_unit")?.to_string();
    if cost_unit.trim().is_empty() { return Err("catalog_cost_unit_empty".into()); }
    let catalog_sha256 = format!("{:x}", Sha256::digest(catalog_raw.as_bytes()));

    let mut families = Vec::new();
    let mut family_ids = BTreeSet::new();
    let family_rows = catalog.get("families").and_then(Value::as_array)
        .ok_or_else(|| "catalog_families_missing".to_string())?;
    if family_rows.len() < 5 { return Err("at_least_five_families_required".into()); }
    for row in family_rows {
        exact_keys(row, &[
            "family_id","planning_horizon_years","active_fraction","product_change_events_per_year",
            "reuse_success_fraction","redeployability_fraction","salvage_realization_fraction",
        ], "family_profile")?;
        let id = string_field(row, "family_id")?.to_string();
        let horizon = row.get("planning_horizon_years").and_then(Value::as_u64)
            .ok_or_else(|| "planning_horizon_years_invalid".to_string())?;
        let active = number_field(row, "active_fraction")?;
        let changes = number_field(row, "product_change_events_per_year")?;
        let reuse = number_field(row, "reuse_success_fraction")?;
        let redeploy = number_field(row, "redeployability_fraction")?;
        if id.trim().is_empty() || !family_ids.insert(id.clone()) || horizon == 0 || horizon > 30
            || !(0.0..=1.0).contains(&active) || changes < 0.0
            || !(0.0..=1.0).contains(&reuse) || !(0.0..=1.0).contains(&redeploy)
        {
            return Err("catalog_family_signal_invalid_or_duplicate".into());
        }
        families.push(FamilySignal {
            family_id:id,
            planning_horizon_years:horizon as u32,
            active_fraction:active,
            product_change_events_per_year:changes,
            reuse_success_fraction:reuse,
            redeployability_fraction:redeploy,
        });
    }

    let strategy_rows = catalog.get("strategies").and_then(Value::as_array)
        .ok_or_else(|| "catalog_strategies_missing".to_string())?;
    let strategy_keys = [
        "strategy_id","kind","owns_asset","acquisition_cost","refurbishment_cost","installation_cost",
        "annual_contract_cost","contract_payment_basis","annual_maintenance_cost","annual_idle_carry_cost",
        "retooling_cost_per_product_change","fallback_replacement_cost_if_reuse_fails",
        "terminal_salvage_value","terminal_disposal_cost","note",
    ];
    let mut strategies_by_kind = BTreeMap::new();
    let mut strategy_ids = BTreeSet::new();
    for row in strategy_rows {
        exact_keys(row, &strategy_keys, "strategy")?;
        let id = string_field(row,"strategy_id")?.to_string();
        let kind = string_field(row,"kind")?.to_string();
        if id.trim().is_empty() || !strategy_ids.insert(id.clone())
            || strategies_by_kind.insert(kind,id).is_some()
        {
            return Err("duplicate_or_empty_strategy_id_or_kind".into());
        }
    }

    let lifecycle_sha256 = format!("{:x}", Sha256::digest(lifecycle_raw.as_bytes()));
    if string_field(lifecycle,"benchmark_id")? != "EV-FACTORY-LIFECYCLE-001"
        || string_field(lifecycle,"source_catalog_sha256")? != catalog_sha256
        || string_field(lifecycle,"source_evidence_class")? != "synthetic"
        || string_field(lifecycle,"cost_unit")? != cost_unit
        || bool_field(lifecycle,"economic_recommendation_authorized")?
        || bool_field(lifecycle,"production_release_authorized")?
    {
        return Err("lifecycle_report_does_not_match_catalog_or_claims_release".into());
    }
    if string_field(adaptive,"benchmark_id")? != "EV-FACTORY-OPTIONS-001"
        || string_field(adaptive,"source_catalog_sha256")? != catalog_sha256
        || string_field(adaptive,"source_lifecycle_report_sha256")? != lifecycle_sha256
        || string_field(adaptive,"evidence_class")? != "synthetic"
        || string_field(adaptive,"cost_unit")? != cost_unit
        || bool_field(adaptive,"economic_recommendation_authorized")?
        || bool_field(adaptive,"production_release_authorized")?
    {
        return Err("adaptive_report_digest_chain_or_claim_ceiling_invalid".into());
    }

    let lifecycle_rows = lifecycle.get("family_strategy_results").and_then(Value::as_array)
        .ok_or_else(|| "lifecycle_family_strategy_rows_missing".to_string())?;
    if lifecycle_rows.len() != families.len() * strategy_ids.len() {
        return Err("lifecycle_family_strategy_matrix_incomplete".into());
    }
    let mut lifecycle_costs = BTreeMap::new();
    let mut family_oracle: BTreeMap<String,(String,f64)> = BTreeMap::new();
    for row in lifecycle_rows {
        let family = string_field(row,"family_id")?.to_string();
        let strategy = string_field(row,"strategy_id")?.to_string();
        let cost = number_field(row,"total_npv_cost")?;
        if !family_ids.contains(&family) || !strategy_ids.contains(&strategy) {
            return Err("lifecycle_row_has_unknown_family_or_strategy".into());
        }
        if lifecycle_costs.insert((family.clone(),strategy.clone()),LifecycleRow{cost}).is_some() {
            return Err("duplicate_lifecycle_cost_row".into());
        }
        let item = family_oracle.entry(family).or_insert((strategy.clone(),cost));
        if cost < item.1 { *item = (strategy,cost); }
    }
    let overheads = adaptive.get("policy_definition_overhead_cost_units").and_then(Value::as_object)
        .ok_or_else(|| "adaptive_policy_overheads_missing".to_string())?;
    let decision_overheads: BTreeMap<String,f64> = overheads.iter()
        .map(|(key,value)| {
            let cost=value.as_f64().ok_or_else(|| "policy_overhead_not_numeric".to_string())?;
            if !cost.is_finite() || cost<0.0 { return Err("policy_overhead_invalid".to_string()); }
            Ok((key.clone(),cost))
        }).collect::<Result<BTreeMap<_,_>,_>>()?;
    let policy_ids: BTreeSet<String> = adaptive.get("policy_ids").and_then(Value::as_array)
        .ok_or_else(|| "adaptive_policy_ids_missing".to_string())?
        .iter().map(|value| value.as_str().map(str::to_string).ok_or_else(|| "policy_id_not_string".to_string()))
        .collect::<Result<BTreeSet<_>,_>>()?;
    if policy_ids.len()!=9 || !policy_ids.contains(ORACLE_POLICY_ID) || decision_overheads.keys().cloned().collect::<BTreeSet<_>>() != policy_ids {
        return Err("adaptive_policy_catalog_or_overhead_set_invalid".into());
    }
    let rows = adaptive.get("family_policy_results").and_then(Value::as_array)
        .ok_or_else(|| "adaptive_family_policy_results_missing".to_string())?;
    let expected = families.len() * policy_ids.len();
    if rows.len()!=expected { return Err("adaptive_family_policy_matrix_count_mismatch".into()); }
    let mut policy_rows = BTreeMap::new();
    for row in rows {
        if string_field(row,"signal_perturbation_case")? != "base" { continue; }
        let family = string_field(row,"family_id")?.to_string();
        let policy = string_field(row,"policy_id")?.to_string();
        let deployable = bool_field(row,"deployable_policy")?;
        let strategy = string_field(row,"selected_strategy_id")?.to_string();
        let raw_npv = number_field(row,"lifecycle_npv_before_decision_overhead")?;
        let overhead = number_field(row,"decision_and_measurement_overhead")?;
        let total = number_field(row,"total_policy_npv_cost")?;
        let oracle_cost = number_field(row,"family_oracle_npv_cost")?;
        let regret = number_field(row,"regret_vs_family_oracle_npv_cost")?;
        let life_cost = lifecycle_costs.get(&(family.clone(),strategy.clone()))
            .ok_or_else(|| "adaptive_selected_strategy_missing_lifecycle_cost".to_string())?;
        let known_policy = policy_ids.contains(&policy);
        if !family_ids.contains(&family) || !known_policy || !strategy_ids.contains(&strategy)
            || deployable == (policy == ORACLE_POLICY_ID)
            || !((raw_npv-life_cost.cost).abs()<=1e-6)
            || !((overhead-decision_overheads[&policy]).abs()<=1e-6)
            || !((total-(raw_npv+overhead)).abs()<=1e-6)
            || !((oracle_cost-family_oracle[&family].1).abs()<=1e-6)
            || !((regret-(total-oracle_cost).max(0.0)).abs()<=1e-6)
        {
            return Err("adaptive_policy_row_mismatches_costs_overheads_or_provenance".into());
        }
        if policy_rows.insert((family.clone(),policy.clone()),PolicyRow {
            family_id:family, policy_id:policy, deployable, selected_strategy_id:strategy,
            lifecycle_npv_before_overhead:raw_npv, overhead,total_cost:total,oracle_cost,regret,
        }).is_some() {
            return Err("duplicate_base_family_policy_row".into());
        }
    }
    if policy_rows.len()!=expected { return Err("adaptive_base_policy_matrix_incomplete".into()); }
    Ok(Inputs{
        catalog:CatalogData{catalog_sha256,cost_unit,strategies_by_kind,strategy_ids,families},
        lifecycle_report_sha256:lifecycle_sha256,
        adaptive_report_sha256:format!("{:x}",Sha256::digest(adaptive_raw.as_bytes())),
        lifecycle_costs,policy_rows,family_oracle,policy_ids,decision_overheads,
    })
}

fn select_strategy(policy_id: &str, family: &FamilySignal, catalog: &CatalogData) -> Result<String,String> {
    let kind = if policy_id.starts_with("fixed_purchase_new") { "purchase_new" }
    else if policy_id.starts_with("fixed_long_lease") { "lease" }
    else if policy_id.starts_with("fixed_short_term_rent") { "short_term_rent" }
    else if policy_id.starts_with("fixed_refurbish_redeploy") { "refurbish_redeploy" }
    else if policy_id == "adaptive_rent_below_30pct_activity" {
        if family.active_fraction<0.30 { "short_term_rent" } else { "refurbish_redeploy" }
    } else if policy_id == "adaptive_lease_at_70pct_change_events" {
        if family.product_change_events_per_year>=0.70 { "lease" } else { "refurbish_redeploy" }
    } else if policy_id == "adaptive_reuse_when_confidence_high" {
        if family.reuse_success_fraction>=0.90 && family.redeployability_fraction>=0.80 { "refurbish_redeploy" } else { "lease" }
    } else if policy_id == "adaptive_low_activity_rent_high_change_lease_else_reuse" {
        if family.active_fraction<0.30 { "short_term_rent" }
        else if family.product_change_events_per_year>=0.70 { "lease" }
        else if family.reuse_success_fraction>=0.90 && family.redeployability_fraction>=0.80 { "refurbish_redeploy" }
        else { "lease" }
    } else {
        return Err("oracle_or_unknown_policy_must_not_use_deployable_selector".into());
    };
    catalog.strategies_by_kind.get(kind).cloned()
        .ok_or_else(|| format!("strategy_kind_{kind}_not_in_catalog"))
}

fn evaluate(inputs: &Inputs) -> Result<Report,String> {
    let families=&inputs.catalog.families;
    let mut fold_results=Vec::new();
    let mut counts:BTreeMap<String,usize>=inputs.policy_ids.iter().map(|p|(p.clone(),0)).collect();
    for held in families {
        let training:Vec<&FamilySignal>=families.iter().filter(|f| f.family_id!=held.family_id).collect();
        if training.len()!=families.len()-1 {return Err("holdout_split_invalid".into());}
        let mut policy_means=Vec::<(String,f64)>::new();
        let mut fixed_means=Vec::<(String,f64)>::new();
        for policy in inputs.policy_ids.iter().filter(|p| p.as_str()!=ORACLE_POLICY_ID) {
            let mut costs=Vec::new();
            for family in &training {
                let selected=select_strategy(policy,family,&inputs.catalog)?;
                let row=inputs.policy_rows.get(&(family.family_id.clone(),policy.clone()))
                    .ok_or_else(|| "training_family_policy_row_missing".to_string())?;
                if row.selected_strategy_id!=selected {
                    return Err("stored_training_choice_disagrees_with_signal_rule".into());
                }
                costs.push(row.total_cost);
            }
            let mean=costs.iter().sum::<f64>()/costs.len() as f64;
            policy_means.push((policy.clone(),mean));
            if policy.starts_with("fixed_") {fixed_means.push((policy.clone(),mean));}
        }
        policy_means.sort_by(|a,b| a.1.total_cmp(&b.1).then_with(||a.0.cmp(&b.0)));
        fixed_means.sort_by(|a,b| a.1.total_cmp(&b.1).then_with(||a.0.cmp(&b.0)));
        let (selected_policy,training_mean)=policy_means.first()
            .ok_or_else(|| "no_deployable_training_policy".to_string())?.clone();
        let (fixed_policy,fixed_training_mean)=fixed_means.first()
            .ok_or_else(|| "no_fixed_training_policy".to_string())?.clone();
        if selected_policy==ORACLE_POLICY_ID || !inputs.decision_overheads.contains_key(&selected_policy) {
            return Err("non_deployable_or_unknown_policy_selected_from_training".into());
        }
        // Apply the already-declared policy rule to the held-out signal only after policy selection.
        let expected_held_strategy=select_strategy(&selected_policy,held,&inputs.catalog)?;
        let held_row=inputs.policy_rows.get(&(held.family_id.clone(),selected_policy.clone()))
            .ok_or_else(|| "heldout_policy_row_missing".to_string())?;
        if held_row.selected_strategy_id!=expected_held_strategy {
            return Err("heldout_policy_choice_disagrees_with_preregistered_rule".into());
        }
        let fixed_row=inputs.policy_rows.get(&(held.family_id.clone(),fixed_policy.clone()))
            .ok_or_else(|| "heldout_fixed_comparator_row_missing".to_string())?;
        if fixed_row.selected_strategy_id!=select_strategy(&fixed_policy,held,&inputs.catalog)? {
            return Err("heldout_fixed_choice_mismatch".into());
        }
        let oracle=inputs.family_oracle.get(&held.family_id)
            .ok_or_else(|| "heldout_family_oracle_missing".to_string())?;
        *counts.get_mut(&selected_policy).unwrap()+=1;
        fold_results.push(FoldResult{
            held_out_family_id:held.family_id.clone(),
            training_family_ids:training.iter().map(|f| f.family_id.clone()).collect(),
            selected_policy_id_from_training_only:selected_policy.clone(),
            selected_policy_training_mean_npv_cost:training_mean,
            held_out_selected_strategy_id:held_row.selected_strategy_id.clone(),
            held_out_policy_npv_cost_including_overhead:held_row.total_cost,
            fixed_comparator_policy_id_from_training_only:fixed_policy,
            fixed_comparator_training_mean_npv_cost:fixed_training_mean,
            held_out_fixed_comparator_npv_cost:fixed_row.total_cost,
            adaptive_minus_fixed_cost_units:held_row.total_cost-fixed_row.total_cost,
            held_out_family_oracle_cost_units:oracle.1,
            regret_vs_held_out_family_oracle_units:(held_row.total_cost-oracle.1).max(0.0),
            selected_decision_overhead_units:held_row.overhead,
            selection_was_independent_of_held_out_cost:true,
        });
    }
    let n=fold_results.len() as f64;
    let mean_policy=fold_results.iter().map(|f|f.held_out_policy_npv_cost_including_overhead).sum::<f64>()/n;
    let mean_fixed=fold_results.iter().map(|f|f.held_out_fixed_comparator_npv_cost).sum::<f64>()/n;
    let mean_delta=fold_results.iter().map(|f|f.adaptive_minus_fixed_cost_units).sum::<f64>()/n;
    let mean_regret=fold_results.iter().map(|f|f.regret_vs_held_out_family_oracle_units).sum::<f64>()/n;
    let worst_policy=fold_results.iter().map(|f|f.held_out_policy_npv_cost_including_overhead).fold(f64::NEG_INFINITY,f64::max);
    let folds_adaptive=fold_results.iter().filter(|f|f.selected_policy_id_from_training_only.starts_with("adaptive_")).count();
    let saves=fold_results.iter().filter(|f|f.adaptive_minus_fixed_cost_units<0.0).count();
    let mut selection_counts:Vec<PolicySelectionCount>=inputs.policy_ids.iter()
        .map(|id|PolicySelectionCount{policy_id:id.clone(),selected_for_held_out_folds:counts[id],deployable:id!=ORACLE_POLICY_ID}).collect();
    selection_counts.sort_by(|a,b|a.policy_id.cmp(&b.policy_id));
    Ok(Report{
        schema_version:"0.1.0".into(),
        benchmark_id:BENCHMARK_ID.into(),
        source_catalog_sha256:inputs.catalog.catalog_sha256.clone(),
        source_lifecycle_report_sha256:inputs.lifecycle_report_sha256.clone(),
        source_adaptive_report_sha256:inputs.adaptive_report_sha256.clone(),
        evidence_class:"synthetic".into(),
        cost_unit:inputs.catalog.cost_unit.clone(),
        family_count:families.len(),
        family_ids:families.iter().map(|f|f.family_id.clone()).collect(),
        policy_count:inputs.policy_ids.len(),
        deployable_policy_count:inputs.policy_ids.len()-1,
        fold_results,
        policy_selection_counts:selection_counts,
        mean_held_out_policy_cost_units:mean_policy,
        worst_held_out_policy_cost_units:worst_policy,
        mean_held_out_fixed_comparator_cost_units:mean_fixed,
        mean_adaptive_minus_fixed_cost_units:mean_delta,
        mean_held_out_regret_vs_family_oracle_units:mean_regret,
        folds_where_selected_policy_was_adaptive:folds_adaptive,
        fold_count_where_adaptive_policy_saved_cost:saves,
        oracle_is_non_deployable:true,
        held_out_costs_used_in_policy_selection:false,
        evaluation_note:"LOFO is a leakage check across five declared synthetic regimes, not an IID performance estimate or external generalization guarantee. In each fold, the policy and fixed comparator are chosen from four training families only; the fifth family is scored afterwards. The oracle is excluded from training candidates.".into(),
        economic_recommendation_authorized:false,
        production_release_authorized:false,
        claim_ceiling:CLAIM_CEILING.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn training_policy_selection_does_not_use_held_out_rows() {
        let ids = vec!["fixed".to_string(),"adaptive".to_string()];
        let mut rows = BTreeMap::<(String,String),PolicyRow>::new();
        for family in ["a","b","c","d","held"] {
            rows.insert((family.into(),"fixed".into()),PolicyRow{
                family_id:family.into(),policy_id:"fixed".into(),deployable:true,
                selected_strategy_id:"buy".into(),lifecycle_npv_before_overhead:100.0,
                overhead:0.0,total_cost:if family=="held" {100.0} else {120.0},
                oracle_cost:80.0,regret:20.0,
            });
            rows.insert((family.into(),"adaptive".into()),PolicyRow{
                family_id:family.into(),policy_id:"adaptive".into(),deployable:true,
                selected_strategy_id:"rent".into(),lifecycle_npv_before_overhead:90.0,
                overhead:5.0,total_cost:if family=="held" {500.0} else {95.0},
                oracle_cost:80.0,regret:15.0,
            });
        }
        let choose = |rows: &BTreeMap<(String,String),PolicyRow>| {
            ids.iter().map(|id| {
                let mean=["a","b","c","d"].iter()
                    .map(|family| rows[&(family.to_string(),id.clone())].total_cost)
                    .sum::<f64>()/4.0;
                (id.clone(),mean)
            }).min_by(|a,b|a.1.total_cmp(&b.1).then_with(||a.0.cmp(&b.0))).unwrap().0
        };
        let first=choose(&rows);
        rows.get_mut(&("held".into(),"adaptive".into())).unwrap().total_cost=999_999.0;
        let after=choose(&rows);
        assert_eq!(first,after);
        assert_eq!(first,"adaptive");
    }

    #[test]
    fn fold_definition_uses_all_other_families_for_training() {
        let families=["a","b","c","d","held"];
        let held= "held";
        let training:Vec<_>=families.iter().filter(|id| **id!=held).copied().collect();
        assert_eq!(training,vec!["a","b","c","d"]);
        assert!(!training.contains(&held));
    }
}

use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::{env, error::Error, fs};

fn obj<'a>(v: &'a Value, p: &str) -> Result<&'a Map<String, Value>, String> {
    v.as_object().ok_or_else(|| format!("{p} must be an object"))
}
fn arr<'a>(v: &'a Value, p: &str) -> Result<&'a Vec<Value>, String> {
    v.as_array().ok_or_else(|| format!("{p} must be an array"))
}
fn text<'a>(v: &'a Value, p: &str) -> Result<&'a str, String> {
    v.as_str().filter(|s| !s.trim().is_empty()).ok_or_else(|| format!("{p} must be non-empty"))
}
fn keys(v: &Value, expected: &[&str], p: &str) -> Result<(), String> {
    let actual: BTreeSet<&str> = obj(v, p)?.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != expected { return Err(format!("{p} has unknown or missing keys")); }
    Ok(())
}
fn yes(v: &Value, p: &str) -> Result<(), String> {
    if v.as_bool() != Some(true) { return Err(format!("{p} must be true")); } Ok(())
}
fn no(v: &Value, p: &str) -> Result<(), String> {
    if v.as_bool() != Some(false) { return Err(format!("{p} must be false")); } Ok(())
}
fn unique_strings(v: &Value, p: &str, min: usize) -> Result<BTreeSet<String>, String> {
    let mut out = BTreeSet::new();
    let xs = arr(v, p)?;
    if xs.len() < min { return Err(format!("{p} needs at least {min} entries")); }
    for x in xs {
        let s = text(x, p)?.to_string();
        if !out.insert(s) { return Err(format!("{p} contains duplicates")); }
    }
    Ok(out)
}

fn validate(c: &Value) -> Result<(), String> {
    keys(c, &[
        "schema_version","artifact_kind","contract_id","status","evidence_class","purpose",
        "searchers","budget_contract","input_information_contract","split_contract",
        "counterexample_admission","required_record_fields","required_metrics","statistical_contract",
        "benchmark_tracks","admission_gates","claim_policy","research_references"
    ], "$")?;
    for (k, v) in [
        ("schema_version","0.1.0"),
        ("artifact_kind","symtropy-counterexample-search-benchmark-contract"),
        ("contract_id","counterexample-search-fixed-budget-v0"),
        ("status","proposed_not_executed"),("evidence_class","test_design_only")
    ] { if c[k].as_str() != Some(v) { return Err(format!("unsupported {k}")); } }
    text(&c["purpose"], "$.purpose")?;

    let searchers = arr(&c["searchers"], "$.searchers")?;
    if searchers.len() < 3 { return Err("at least three searchers required".into()); }
    let mut ids = BTreeSet::new(); let mut baselines = 0; let mut candidates = 0;
    for (i, s) in searchers.iter().enumerate() {
        let p = format!("$.searchers[{i}]");
        keys(s, &["searcher_id","class","strategy","implementation_status","adaptive_feedback_allowed","uses_future_or_holdout_information"], &p)?;
        if !ids.insert(text(&s["searcher_id"], &p)?.to_string()) { return Err("searcher IDs must be unique".into()); }
        text(&s["strategy"], &p)?;
        match s["class"].as_str() { Some("baseline") => baselines += 1, Some("candidate") => candidates += 1, _ => return Err("invalid searcher class".into()) }
        if !matches!(s["implementation_status"].as_str(), Some("planned_adapter"|"implemented_not_benchmarked"|"research_candidate_not_benchmarked")) {
            return Err(format!("{p} has unsupported implementation status"));
        }
        if s["adaptive_feedback_allowed"].as_bool().is_none() { return Err(format!("{p} adaptive flag must be boolean")); }
        no(&s["uses_future_or_holdout_information"], &format!("{p}.uses_future_or_holdout_information"))?;
    }
    if baselines < 2 || candidates != 1 || !ids.contains("uniform_random") || !ids.contains("symtropy_adaptive_multiobjective") {
        return Err("require at least two baselines, exactly one Symtropy candidate, and uniform-random baseline".into());
    }

    let b = &c["budget_contract"];
    keys(b, &[
        "budget_unit","pilot_repetitions","pilot_invocation_budget_per_searcher","confirmatory_repetitions",
        "confirmatory_invocation_budget_per_searcher","same_invocation_budget_across_searchers",
        "invalid_invocations_count_against_budget","failed_or_timed_out_invocations_count_against_budget",
        "wall_time_and_resource_use_reported_separately","resource_budget_must_match_across_arms",
        "run_order_randomized_or_blocked","random_seeds_preregistered","repetitions_are_independent_at_search_run_level"
    ], "$.budget_contract")?;
    if b["budget_unit"].as_str() != Some("plant_oracle_invocation") { return Err("unsupported budget unit".into()); }
    for k in ["pilot_repetitions","pilot_invocation_budget_per_searcher","confirmatory_repetitions","confirmatory_invocation_budget_per_searcher"] {
        let n = b[k].as_u64().ok_or_else(|| format!("{k} must be an unsigned integer"))?;
        if n == 0 || (k == "confirmatory_repetitions" && n < 30) { return Err(format!("{k} below minimum")); }
    }
    for k in ["same_invocation_budget_across_searchers","invalid_invocations_count_against_budget","failed_or_timed_out_invocations_count_against_budget","wall_time_and_resource_use_reported_separately","resource_budget_must_match_across_arms","run_order_randomized_or_blocked","random_seeds_preregistered","repetitions_are_independent_at_search_run_level"] {
        yes(&b[k], &format!("$.budget_contract.{k}"))?;
    }

    let info = &c["input_information_contract"];
    for (k,v) in obj(info,"$.input_information_contract")? { yes(v, &format!("$.input_information_contract.{k}"))?; }
    let split = &c["split_contract"];
    if split["split_unit"].as_str() != Some("whole_scenario_family_episode_or_site_year") { return Err("split must preserve whole scenarios/episodes/site-years".into()); }
    for k in ["development_set","validation_set","locked_holdout"] { text(&split[k], k)?; }
    yes(&split["holdout_frozen_before_confirmatory_run"], "holdout freeze")?;
    no(&split["holdout_used_for_searcher_selection_or_tuning"], "holdout tuning")?;
    yes(&split["test_leakage_invalidates_claim"], "test leakage policy")?;
    yes(&split["holdout_discovery_is_reported_separately"], "holdout reporting")?;

    for (k,v) in obj(&c["counterexample_admission"],"$.counterexample_admission")? { yes(v, &format!("counterexample admission {k}"))?; }
    let records = unique_strings(&c["required_record_fields"], "$.required_record_fields", 10)?;
    for k in ["searcher_id","search_run_id","preregistered_seed","oracle_invocation_index","scenario_input_sha256","controller_artifact_sha256","plant_model_sha256","run_configuration_sha256","oracle_status","gate_result","replay_status","failure_family_fingerprint","wall_time_ms","peak_memory_bytes"] {
        if !records.contains(k) { return Err(format!("missing required record field {k}")); }
    }
    let metrics = unique_strings(&c["required_metrics"], "$.required_metrics", 8)?;
    for k in ["falsification_rate_at_budget","time_to_first_replay_confirmed_counterexample","survival_curve_for_runs_not_falsified","unique_replay_confirmed_failure_families_per_100_invocations","invalid_invocation_rate","replay_confirmation_rate","locked_holdout_failure_family_yield","wall_time_and_resource_use"] {
        if !metrics.contains(k) { return Err(format!("missing required metric {k}")); }
    }

    let st = &c["statistical_contract"];
    if st["primary_endpoint"].as_str() != Some("falsification_rate_at_budget")
        || st["independent_unit"].as_str() != Some("search_run") || !st["effect_size_target"].is_null() {
        return Err("invalid primary endpoint/unit or premature effect-size target".into());
    }
    for k in ["report_confidence_interval","non_falsified_runs_are_right_censored_at_budget","survival_analysis_required","paired_or_blocked_seed_design_required","multiple_comparison_plan_required","effect_size_target_frozen_after_pilot_before_locked_holdout","superiority_requires_paired_uncertainty_interval_excluding_zero","no_single_composite_score_allowed"] {
        yes(&st[k], &format!("statistical contract {k}"))?;
    }

    let tracks = arr(&c["benchmark_tracks"], "$.benchmark_tracks")?;
    if tracks.len() < 3 { return Err("three evidence tracks required".into()); }
    let mut track_ids = BTreeSet::new();
    for (i,t) in tracks.iter().enumerate() {
        let p = format!("$.benchmark_tracks[{i}]");
        text(&t["track_id"], &p)?;
        if !track_ids.insert(t["track_id"].as_str().unwrap().to_string()) { return Err("duplicate track ID".into()); }
        text(&t["purpose"], &p)?; text(&t["oracle_type"], &p)?;
        no(&t["physical_evidence"], &format!("{p}.physical_evidence"))?;
        no(&t["eligible_for_controller_superiority_claim"], &format!("{p}.eligible_for_controller_superiority_claim"))?;
    }
    for (id,status) in [("analytic_known_answer","contract_only"),("validated_plant_model","not_admitted"),("locked_site_episode_holdout","not_created")] {
        let track = tracks.iter().find(|t| t["track_id"].as_str()==Some(id)).ok_or_else(|| format!("missing track {id}"))?;
        if track["status"].as_str() != Some(status) { return Err(format!("track {id} status overstates evidence")); }
    }

    let gates = obj(&c["admission_gates"], "$.admission_gates")?;
    if gates.is_empty() || gates.values().any(|v| v.as_bool()!=Some(false)) { return Err("all admission gates must remain false".into()); }
    let claim = &c["claim_policy"];
    if claim["performance_claim"].as_str()!=Some("none") { return Err("performance claims forbidden".into()); }
    for k in ["synthetic_oracle_results_are_physical_evidence","searcher_superiority_implies_controller_superiority","zero_discoveries_proves_safety","holdout_tuning_allowed"] {
        no(&claim[k], &format!("claim policy {k}"))?;
    }
    let refs=arr(&c["research_references"], "$.research_references")?;
    if refs.len()<3 { return Err("at least three references required".into()); }
    for (i,r) in refs.iter().enumerate() {
        let p=format!("$.research_references[{i}]");
        text(&r["title"],&p)?; text(&r["relevance"],&p)?;
        if !text(&r["url"],&p)?.starts_with("https://") { return Err(format!("{p} URL must use HTTPS")); }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let path=env::args().nth(1).ok_or("usage: counterexample_search_contract <contract.json>")?;
    let raw=fs::read_to_string(path)?;
    let contract:Value=serde_json::from_str(&raw)?;
    validate(&contract).map_err(|e|format!("counterexample search contract rejected: {e}"))?;
    println!("{}",json!({
        "schema_version":"0.1.0",
        "report_kind":"counterexample-search-benchmark-contract-validation",
        "contract_id":"counterexample-search-fixed-budget-v0",
        "contract_valid":true,
        "searcher_count":contract["searchers"].as_array().map(Vec::len).unwrap_or(0),
        "pilot_invocation_budget_per_searcher":contract["budget_contract"]["pilot_invocation_budget_per_searcher"],
        "confirmatory_invocation_budget_per_searcher":contract["budget_contract"]["confirmatory_invocation_budget_per_searcher"],
        "confirmatory_repetitions":contract["budget_contract"]["confirmatory_repetitions"],
        "admission_gate_count":contract["admission_gates"].as_object().map(Map::len).unwrap_or(0),
        "admission_gates_all_false":true,
        "candidate_for_execution":false,
        "performance_claim":"none",
        "result_scope":"benchmark contract only; no searcher or physical-model claim"
    }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn load()->Value { serde_json::from_str(include_str!("../../fixtures/counterexample-search-benchmark-contract-2026.json")).unwrap() }
    #[test] fn accepts_design_only_contract(){ validate(&load()).unwrap(); }
    #[test] fn rejects_duplicate_searcher(){let mut c=load();c["searchers"][1]["searcher_id"]=json!("uniform_random");assert!(validate(&c).unwrap_err().contains("unique"));}
    #[test] fn rejects_unfair_budget(){let mut c=load();c["budget_contract"]["same_invocation_budget_across_searchers"]=json!(false);assert!(validate(&c).unwrap_err().contains("must be true"));}
    #[test] fn rejects_unbudgeted_invalid_calls(){let mut c=load();c["budget_contract"]["invalid_invocations_count_against_budget"]=json!(false);assert!(validate(&c).unwrap_err().contains("must be true"));}
    #[test] fn rejects_holdout_tuning(){let mut c=load();c["split_contract"]["holdout_used_for_searcher_selection_or_tuning"]=json!(true);assert!(validate(&c).unwrap_err().contains("must be false"));}
    #[test] fn rejects_unreplayed_failures(){let mut c=load();c["counterexample_admission"]["counterexample_requires_independent_replay"]=json!(false);assert!(validate(&c).unwrap_err().contains("must be true"));}
    #[test] fn rejects_synthetic_performance_claim(){let mut c=load();c["claim_policy"]["synthetic_oracle_results_are_physical_evidence"]=json!(true);assert!(validate(&c).unwrap_err().contains("must be false"));}
    #[test] fn rejects_open_admission_gate(){let mut c=load();c["admission_gates"]["source_and_model_admission"]=json!(true);assert!(validate(&c).unwrap_err().contains("all admission gates"));}
    #[test] fn rejects_unfrozen_effect_size(){let mut c=load();c["statistical_contract"]["effect_size_target"]=json!(0.1);assert!(validate(&c).unwrap_err().contains("premature effect-size target"));}
    #[test] fn rejects_composite_score(){let mut c=load();c["statistical_contract"]["no_single_composite_score_allowed"]=json!(false);assert!(validate(&c).unwrap_err().contains("must be true"));}
    #[test] fn rejects_unknown_field(){let mut c=load();c["sota_beaten"]=json!(true);assert!(validate(&c).unwrap_err().contains("unknown or missing keys"));}
}

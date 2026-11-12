//! Evidence-gated evaluation of multi-source transfer and negative transfer.
//!
//! This crate audits a frozen result bundle. It does not train a transfer model.
//! It keeps source eligibility, domain/task compatibility, learned weights and
//! held-out source-ablation utility as separate evidence dimensions.
#![forbid(unsafe_code)]

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

type Check<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct Metrics {
    macro_recall: f64,
    worst_fault_recall: f64,
    false_alarms_per_engine_hour: f64,
    ece: f64,
    unknown_false_known_rate: f64,
}

impl Metrics {
    fn read(v: &Value, at: &str) -> Check<Self> {
        exact_keys(v, &[
            "macro_recall",
            "worst_fault_recall",
            "false_alarms_per_engine_hour",
            "ece",
            "unknown_false_known_rate",
        ], at)?;
        let result = Self {
            macro_recall: number(v, "macro_recall", at)?,
            worst_fault_recall: number(v, "worst_fault_recall", at)?,
            false_alarms_per_engine_hour: number(v, "false_alarms_per_engine_hour", at)?,
            ece: number(v, "ece", at)?,
            unknown_false_known_rate: number(v, "unknown_false_known_rate", at)?,
        };
        for (name, value) in [
            ("macro_recall", result.macro_recall),
            ("worst_fault_recall", result.worst_fault_recall),
            ("ece", result.ece),
            ("unknown_false_known_rate", result.unknown_false_known_rate),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(format!("{at}.{name}: expected value in [0,1]"));
            }
        }
        if result.false_alarms_per_engine_hour < 0.0 {
            return Err(format!("{at}.false_alarms_per_engine_hour: must be non-negative"));
        }
        Ok(result)
    }

    fn deltas_from(&self, without: &Self) -> Value {
        let d = [
            self.macro_recall - without.macro_recall,
            self.worst_fault_recall - without.worst_fault_recall,
            self.false_alarms_per_engine_hour - without.false_alarms_per_engine_hour,
            self.ece - without.ece,
            self.unknown_false_known_rate - without.unknown_false_known_rate,
        ];
        let negative = d[0] < -1e-9
            || d[1] < -1e-9
            || d[2] > 1e-9
            || d[3] > 1e-9
            || d[4] > 1e-9;
        json!({
            "delta_macro_recall": d[0],
            "delta_worst_fault_recall": d[1],
            "delta_false_alarms_per_engine_hour": d[2],
            "delta_ece": d[3],
            "delta_unknown_false_known_rate": d[4],
            "negative_transfer_on_one_or_more_metrics": negative
        })
    }

    fn differences_from(&self, baseline: &Self) -> Value {
        json!({
            "delta_macro_recall": self.macro_recall - baseline.macro_recall,
            "delta_worst_fault_recall": self.worst_fault_recall - baseline.worst_fault_recall,
            "delta_false_alarms_per_engine_hour": self.false_alarms_per_engine_hour - baseline.false_alarms_per_engine_hour,
            "delta_ece": self.ece - baseline.ece,
            "delta_unknown_false_known_rate": self.unknown_false_known_rate - baseline.unknown_false_known_rate
        })
    }

    fn to_json(&self) -> Value {
        json!({
            "macro_recall": self.macro_recall,
            "worst_fault_recall": self.worst_fault_recall,
            "false_alarms_per_engine_hour": self.false_alarms_per_engine_hour,
            "ece": self.ece,
            "unknown_false_known_rate": self.unknown_false_known_rate
        })
    }
}

#[derive(Clone, Debug)]
struct Source {
    source_id: String,
    logical_source_id: String,
    data_sha256: String,
    engine_id: String,
    domain_id: String,
    source_role: String,
    eligible: bool,
    exclusion_reasons: Vec<String>,
}

fn object<'a>(v: &'a Value, at: &str) -> Check<&'a serde_json::Map<String, Value>> {
    v.as_object().ok_or_else(|| format!("{at}: expected object"))
}

fn exact_keys(v: &Value, keys: &[&str], at: &str) -> Check<()> {
    let m = object(v, at)?;
    let expected: BTreeSet<&str> = keys.iter().copied().collect();
    let actual: BTreeSet<&str> = m.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected.difference(&actual).copied().collect();
    let unknown: Vec<_> = actual.difference(&expected).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!("{at}: missing keys {missing:?}; unknown keys {unknown:?}"));
    }
    Ok(())
}

fn field<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(v, at)?.get(key).ok_or_else(|| format!("{at}: missing {key}"))
}

fn text<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    field(v, key, at)?
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{at}.{key}: expected non-empty string"))
}

fn number(v: &Value, key: &str, at: &str) -> Check<f64> {
    let x = field(v, key, at)?
        .as_f64()
        .ok_or_else(|| format!("{at}.{key}: expected number"))?;
    if !x.is_finite() {
        return Err(format!("{at}.{key}: non-finite number"));
    }
    Ok(x)
}

fn boolean(v: &Value, key: &str, at: &str) -> Check<bool> {
    field(v, key, at)?
        .as_bool()
        .ok_or_else(|| format!("{at}.{key}: expected boolean"))
}

fn strings(v: &Value, key: &str, at: &str) -> Check<Vec<String>> {
    let arr = field(v, key, at)?
        .as_array()
        .ok_or_else(|| format!("{at}.{key}: expected array"))?;
    let mut out = Vec::with_capacity(arr.len());
    let mut seen = BTreeSet::new();
    for item in arr {
        let s = item.as_str().filter(|x| !x.trim().is_empty())
            .ok_or_else(|| format!("{at}.{key}: expected non-empty strings"))?;
        if !seen.insert(s.to_owned()) {
            return Err(format!("{at}.{key}: duplicate entry {s}"));
        }
        out.push(s.to_owned());
    }
    Ok(out)
}

fn sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn hex40(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn metric_run(v: &Value, at: &str) -> Check<Metrics> {
    Metrics::read(field(v, "metrics", at)?, &format!("{at}.metrics"))
}

fn validate_source(
    v: &Value,
    at: &str,
    evidence_class: &str,
    target_taxonomy: &str,
    _target_engine: &str,
) -> Check<Source> {
    exact_keys(v, &[
        "source_id",
        "logical_source_id",
        "source_data_sha256",
        "engine_id",
        "domain_id",
        "source_role",
        "evidence_state",
        "label_state",
        "fault_taxonomy_sha256",
        "taxonomy_compatibility",
        "taxonomy_mapping_sha256",
        "quality_state",
        "calibration_state",
        "duplicate_of",
    ], at)?;
    let source_id = text(v, "source_id", at)?.to_owned();
    let logical_source_id = text(v, "logical_source_id", at)?.to_owned();
    let data_sha = text(v, "source_data_sha256", at)?.to_owned();
    if !sha256(&data_sha) {
        return Err(format!("{at}.source_data_sha256: invalid SHA-256"));
    }
    let engine_id = text(v, "engine_id", at)?.to_owned();
    let domain_id = text(v, "domain_id", at)?.to_owned();
    let role = text(v, "source_role", at)?.to_owned();
    if !matches!(role.as_str(), "MEASURED" | "SIMULATED" | "DERIVED" | "RECONSTRUCTED") {
        return Err(format!("{at}.source_role: unsupported source role"));
    }
    let evidence = text(v, "evidence_state", at)?;
    let label = text(v, "label_state", at)?;
    let taxonomy = text(v, "fault_taxonomy_sha256", at)?;
    if !sha256(taxonomy) {
        return Err(format!("{at}.fault_taxonomy_sha256: invalid SHA-256"));
    }
    let compatibility = text(v, "taxonomy_compatibility", at)?;
    if !matches!(compatibility, "EXACT" | "APPROVED_MAPPING" | "INCOMPATIBLE") {
        return Err(format!("{at}.taxonomy_compatibility: unsupported value"));
    }
    let mapping = field(v, "taxonomy_mapping_sha256", at)?;
    let mapping_valid = mapping.as_str().is_some_and(sha256);
    if compatibility == "APPROVED_MAPPING" && !mapping_valid {
        return Err(format!("{at}.taxonomy_mapping_sha256: approved mapping needs a SHA-256"));
    }
    if compatibility != "APPROVED_MAPPING" && !mapping.is_null() {
        return Err(format!("{at}.taxonomy_mapping_sha256: only approved mappings may carry a mapping digest"));
    }
    if compatibility == "EXACT" && taxonomy != target_taxonomy {
        return Err(format!("{at}: EXACT taxonomy compatibility has a different taxonomy SHA"));
    }
    if compatibility == "INCOMPATIBLE" && taxonomy == target_taxonomy {
        return Err(format!("{at}: incompatible taxonomy cannot have the target taxonomy SHA"));
    }
    let quality = text(v, "quality_state", at)?;
    let calibration = text(v, "calibration_state", at)?;
    if !matches!(quality, "CURRENT" | "STALE" | "REJECTED" | "UNKNOWN") {
        return Err(format!("{at}.quality_state: unsupported value"));
    }
    if !matches!(calibration, "CURRENT" | "NOT_APPLICABLE" | "EXPIRED" | "UNKNOWN") {
        return Err(format!("{at}.calibration_state: unsupported value"));
    }
    let duplicate = field(v, "duplicate_of", at)?;
    let is_duplicate = !duplicate.is_null();
    if is_duplicate && duplicate.as_str().is_none_or(|s| s.trim().is_empty()) {
        return Err(format!("{at}.duplicate_of: expected null or source ID"));
    }
    let mut reasons = Vec::new();
    let expected_evidence = if evidence_class == "SYNTHETIC_KNOWN_TRUTH" {
        evidence == "SYNTHETIC_ONLY" && label == "SYNTHETIC_KNOWN_TRUTH"
    } else {
        evidence == "QUALIFIED" && label == "INDEPENDENTLY_ADJUDICATED"
    };
    if !expected_evidence {
        reasons.push("SOURCE_EVIDENCE_OR_LABELS_NOT_QUALIFIED".to_owned());
    }
    if compatibility == "INCOMPATIBLE" {
        reasons.push("FAULT_TAXONOMY_INCOMPATIBLE".to_owned());
    }
    if quality != "CURRENT" {
        reasons.push("SOURCE_QUALITY_NOT_CURRENT".to_owned());
    }
    if calibration != "CURRENT" && calibration != "NOT_APPLICABLE" {
        reasons.push("SOURCE_CALIBRATION_NOT_CURRENT".to_owned());
    }
    if is_duplicate {
        reasons.push("DUPLICATE_COPY_NOT_AN_INDEPENDENT_SOURCE".to_owned());
    }
    Ok(Source {
        source_id,
        logical_source_id,
        data_sha256: data_sha,
        engine_id,
        domain_id,
        source_role: role,
        eligible: reasons.is_empty(),
        exclusion_reasons: reasons,
    })
}

fn validate_scope(claim_scope: &str, target_engine: &str, selected: &[&Source]) -> Check<()> {
    let source_engines: BTreeSet<&str> = selected.iter().map(|s| s.engine_id.as_str()).collect();
    let source_roles: BTreeSet<&str> = selected.iter().map(|s| s.source_role.as_str()).collect();
    match claim_scope {
        "WITHIN_ENGINE_CROSS_LOAD" => {
            if selected.is_empty() || source_engines.iter().any(|id| *id != target_engine) {
                return Err("WITHIN_ENGINE_CROSS_LOAD requires sources from the target engine only".to_owned());
            }
            if selected.iter().all(|s| s.domain_id == selected[0].domain_id) {
                return Err("WITHIN_ENGINE_CROSS_LOAD requires distinct source operating domains".to_owned());
            }
        }
        "CROSS_ENGINE" => {
            if source_engines.len() < 2 || source_engines.contains(target_engine) {
                return Err("CROSS_ENGINE requires at least two source engines distinct from the sealed target engine".to_owned());
            }
        }
        "REAL_SIMULATION_HYBRID" => {
            if !source_roles.contains("MEASURED") || !source_roles.contains("SIMULATED") {
                return Err("REAL_SIMULATION_HYBRID requires at least one measured and one simulated source".to_owned());
            }
        }
        _ => return Err("unsupported claim_scope".to_owned()),
    }
    Ok(())
}

fn evaluate(case: &Value, source_commit: &str, case_sha: &str) -> Check<Value> {
    if !hex40(source_commit) || !sha256(case_sha) {
        return Err("invalid source commit or case SHA-256".to_owned());
    }
    exact_keys(case, &[
        "schema_version",
        "benchmark_id",
        "evidence_class",
        "claim_scope",
        "target",
        "contract",
        "sources",
        "method_runs",
        "claim_ceiling",
    ], "$")?;
    if text(case, "schema_version", "$")? != "0.1.0" {
        return Err("unsupported schema version".to_owned());
    }
    let evidence_class = text(case, "evidence_class", "$")?;
    if !matches!(evidence_class, "SYNTHETIC_KNOWN_TRUTH" | "MEASURED_BENCHMARK") {
        return Err("unsupported evidence class".to_owned());
    }
    let claim_scope = text(case, "claim_scope", "$")?;
    let target = field(case, "target", "$")?;
    exact_keys(target, &[
        "asset_id",
        "engine_id",
        "configuration_sha256",
        "target_domain_id",
        "fault_taxonomy_sha256",
        "target_split_sha256",
        "test_labels_sealed",
        "weight_selection_target_labels_allowed",
    ], "$.target")?;
    let target_engine = text(target, "engine_id", "$.target")?;
    let target_taxonomy = text(target, "fault_taxonomy_sha256", "$.target")?;
    let target_split = text(target, "target_split_sha256", "$.target")?;
    for (key, value) in [
        ("configuration_sha256", text(target, "configuration_sha256", "$.target")?),
        ("fault_taxonomy_sha256", target_taxonomy),
        ("target_split_sha256", target_split),
    ] {
        if !sha256(value) {
            return Err(format!("$.target.{key}: invalid SHA-256"));
        }
    }
    if !boolean(target, "test_labels_sealed", "$.target")?
        || boolean(target, "weight_selection_target_labels_allowed", "$.target")? {
        return Err("target test labels must remain sealed and unavailable to source-weight selection".to_owned());
    }

    let contract = field(case, "contract", "$")?;
    exact_keys(contract, &[
        "group_key",
        "preprocessing_fit_scope",
        "weight_selection_scope",
        "overlapping_windows_across_splits",
        "matched_compute_budget_required",
        "matched_seed_required",
        "metrics",
    ], "$.contract")?;
    let group_key = text(contract, "group_key", "$.contract")?;
    if !matches!(group_key, "ENGINE_CAMPAIGN_RUN" | "PHYSICAL_ENGINE" | "VESSEL") {
        return Err("unsupported split grouping key".to_owned());
    }
    if text(contract, "preprocessing_fit_scope", "$.contract")? != "SOURCE_TRAIN_ONLY"
        || text(contract, "weight_selection_scope", "$.contract")? != "SOURCE_TRAIN_PLUS_UNLABELED_TARGET" {
        return Err("preprocessing or weight selection has access beyond the declared training/unlabeled-target scope".to_owned());
    }
    if boolean(contract, "overlapping_windows_across_splits", "$.contract")?
        || !boolean(contract, "matched_compute_budget_required", "$.contract")?
        || !boolean(contract, "matched_seed_required", "$.contract")? {
        return Err("split leakage or unmatched comparison budget is not permitted".to_owned());
    }
    let metric_names = strings(contract, "metrics", "$.contract")?;
    let expected_metrics = [
        "macro_recall",
        "worst_fault_recall",
        "false_alarms_per_engine_hour",
        "ece",
        "unknown_false_known_rate",
    ];
    if metric_names.len() != expected_metrics.len()
        || expected_metrics.iter().any(|m| !metric_names.iter().any(|x| x == m)) {
        return Err("benchmark must retain all five predeclared non-compensatory metrics".to_owned());
    }

    let source_items = field(case, "sources", "$")?
        .as_array()
        .ok_or_else(|| "$.sources: expected array".to_owned())?;
    if source_items.is_empty() || source_items.len() > 100 {
        return Err("source count outside 1..=100".to_owned());
    }
    let mut sources = BTreeMap::new();
    let mut source_exclusions = Vec::new();
    for (index, item) in source_items.iter().enumerate() {
        let at = format!("$.sources[{index}]");
        let source = validate_source(item, &at, evidence_class, target_taxonomy, target_engine)?;
        if sources.insert(source.source_id.clone(), source.clone()).is_some() {
            return Err(format!("{at}: duplicate source ID"));
        }
        if !source.eligible {
            source_exclusions.push(json!({
                "source_id": source.source_id,
                "logical_source_id": source.logical_source_id,
                "reason_codes": source.exclusion_reasons
            }));
        }
    }
    for source in sources.values() {
        if let Some(parent) = source_items.iter()
            .find(|v| v.get("source_id").and_then(Value::as_str) == Some(&source.source_id))
            .and_then(|v| v.get("duplicate_of"))
            .and_then(Value::as_str) {
            let original = sources.get(parent).ok_or_else(|| format!("{}: duplicate_of refers to unknown source", source.source_id))?;
            if original.logical_source_id != source.logical_source_id || original.data_sha256 != source.data_sha256 {
                return Err(format!("{}: duplicate copy must share logical source ID and data digest with its parent", source.source_id));
            }
        }
    }

    let runs = field(case, "method_runs", "$")?
        .as_array()
        .ok_or_else(|| "$.method_runs: expected array".to_owned())?;
    let mut method_map: BTreeMap<String, (&Value, Metrics)> = BTreeMap::new();
    let mut run_ids = BTreeSet::new();
    let mut reference_split = None;
    let mut reference_seed = None;
    let mut reference_label_budget = None;
    let mut reference_compute_budget = None;
    for (index, run) in runs.iter().enumerate() {
        let at = format!("$.method_runs[{index}]");
        exact_keys(run, &[
            "run_id",
            "method_id",
            "source_ids",
            "source_weights",
            "target_split_sha256",
            "seed",
            "label_budget",
            "compute_budget_id",
            "target_test_labels_used",
            "preprocessing_fit_scope",
            "weight_selection_scope",
            "weighting_rule_id",
            "metrics",
            "source_ablations",
        ], &at)?;
        let run_id = text(run, "run_id", &at)?;
        if !run_ids.insert(run_id.to_owned()) {
            return Err(format!("{at}: duplicate run ID"));
        }
        let method = text(run, "method_id", &at)?.to_owned();
        if !matches!(method.as_str(), "NO_TRANSFER" | "BEST_SINGLE_SOURCE" | "UNIFORM_MULTI_SOURCE" | "ENTROPY_WEIGHTED_MULTI_SOURCE") {
            return Err(format!("{at}.method_id: unsupported method"));
        }
        if method_map.contains_key(&method) {
            return Err(format!("{at}: duplicate method run {method}"));
        }
        let split = text(run, "target_split_sha256", &at)?;
        if !sha256(split) || split != target_split {
            return Err(format!("{at}: target split does not bind the case target split"));
        }
        let seed = field(run, "seed", &at)?.as_u64().ok_or_else(|| format!("{at}.seed: expected non-negative integer"))?;
        let label_budget = field(run, "label_budget", &at)?.as_u64().ok_or_else(|| format!("{at}.label_budget: expected non-negative integer"))?;
        let compute_budget = text(run, "compute_budget_id", &at)?.to_owned();
        if reference_split.is_none() {
            reference_split = Some(split.to_owned());
            reference_seed = Some(seed);
            reference_label_budget = Some(label_budget);
            reference_compute_budget = Some(compute_budget.clone());
        }
        if reference_split.as_deref() != Some(split)
            || reference_seed != Some(seed)
            || reference_label_budget != Some(label_budget)
            || reference_compute_budget.as_deref() != Some(compute_budget.as_str()) {
            return Err(format!("{at}: methods do not share the exact target split, seed, label budget and compute budget"));
        }
        if boolean(run, "target_test_labels_used", &at)?
            || text(run, "preprocessing_fit_scope", &at)? != "SOURCE_TRAIN_ONLY"
            || text(run, "weight_selection_scope", &at)? != "SOURCE_TRAIN_PLUS_UNLABELED_TARGET" {
            return Err(format!("{at}: target-test labels or unauthorized preprocessing/weight-selection scope detected"));
        }
        let source_ids = strings(run, "source_ids", &at)?;
        let weights = field(run, "source_weights", &at)?
            .as_array()
            .ok_or_else(|| format!("{at}.source_weights: expected array"))?;
        if source_ids.len() != weights.len() {
            return Err(format!("{at}: source IDs and source weights must align exactly"));
        }
        let mut selected = Vec::new();
        let mut seen_logical = BTreeSet::new();
        let mut seen_data = BTreeSet::new();
        let mut weight_sum = 0.0;
        let mut weight_by_source = BTreeMap::new();
        for (wi, weight) in weights.iter().enumerate() {
            let w_at = format!("{at}.source_weights[{wi}]");
            exact_keys(weight, &["source_id", "weight"], &w_at)?;
            let source_id = text(weight, "source_id", &w_at)?;
            if source_id != source_ids[wi] {
                return Err(format!("{w_at}: source ID does not match source_ids ordering"));
            }
            let source = sources.get(source_id).ok_or_else(|| format!("{w_at}: unknown source ID"))?;
            if !source.eligible {
                return Err(format!("{w_at}: ineligible source cannot receive a transfer weight"));
            }
            if !seen_logical.insert(source.logical_source_id.clone())
                || !seen_data.insert(source.data_sha256.clone()) {
                return Err(format!("{w_at}: duplicated logical source or repeated source-data digest cannot be counted as independent evidence"));
            }
            let w = number(weight, "weight", &w_at)?;
            if !(0.0..=1.0).contains(&w) || w <= 0.0 {
                return Err(format!("{w_at}.weight: selected source weight must be in (0,1]"));
            }
            weight_sum += w;
            weight_by_source.insert(source_id.to_owned(), w);
            selected.push(source);
        }
        if (weight_sum - if source_ids.is_empty() { 0.0 } else { 1.0 }).abs() > 1e-9 {
            return Err(format!("{at}: source weights must sum to 1 (or be empty for NO_TRANSFER)"));
        }
        match method.as_str() {
            "NO_TRANSFER" if !source_ids.is_empty() => return Err(format!("{at}: NO_TRANSFER must not select sources")),
            "NO_TRANSFER" => {},
            "BEST_SINGLE_SOURCE" if source_ids.len() != 1 || (weight_by_source.values().next().copied().unwrap_or(0.0) - 1.0).abs() > 1e-9 =>
                return Err(format!("{at}: BEST_SINGLE_SOURCE must select exactly one source with weight 1")),
            "UNIFORM_MULTI_SOURCE" => {
                if source_ids.len() < 2 {
                    return Err(format!("{at}: uniform fusion requires at least two sources"));
                }
                let expected = 1.0 / source_ids.len() as f64;
                if weight_by_source.values().any(|w| (w - expected).abs() > 1e-9) {
                    return Err(format!("{at}: UNIFORM_MULTI_SOURCE weights must be equal"));
                }
            }
            "ENTROPY_WEIGHTED_MULTI_SOURCE" => {
                if source_ids.len() < 2 || text(run, "weighting_rule_id", &at)? != "EW_MDAN_REPRODUCTION_CONTRACT_V0_1" {
                    return Err(format!("{at}: entropy-weighted run requires at least two sources and the frozen weighting-rule identity"));
                }
            }
            _ => {}
        }
        if method != "ENTROPY_WEIGHTED_MULTI_SOURCE" && !field(run, "weighting_rule_id", &at)?.is_null() {
            return Err(format!("{at}: only entropy-weighted method may carry a dynamic weighting rule"));
        }
        if !selected.is_empty() {
            validate_scope(claim_scope, target_engine, &selected)?;
        }
        let metrics = metric_run(run, &at)?;
        method_map.insert(method, (run, metrics));
    }

    for method in ["NO_TRANSFER", "BEST_SINGLE_SOURCE", "UNIFORM_MULTI_SOURCE", "ENTROPY_WEIGHTED_MULTI_SOURCE"] {
        if !method_map.contains_key(method) {
            return Err(format!("missing required matched comparator {method}"));
        }
    }
    if method_map.len() != 4 {
        return Err("unexpected additional methods in v0.1 comparator tournament".to_owned());
    }

    let (dynamic_run, dynamic_metrics) = method_map.get("ENTROPY_WEIGHTED_MULTI_SOURCE")
        .ok_or_else(|| "missing entropy-weighted comparator".to_owned())?;
    let selected_ids = strings(dynamic_run, "source_ids", "$.method_runs.entropy")?;
    let ablations = field(dynamic_run, "source_ablations", "$.method_runs.entropy")?
        .as_array().ok_or_else(|| "$.method_runs.entropy.source_ablations: expected array".to_owned())?;
    if ablations.len() != selected_ids.len() {
        return Err("each selected entropy-weighted source needs exactly one leave-one-source-out ablation".to_owned());
    }
    let mut ablation_map = BTreeMap::new();
    for (i, ablation) in ablations.iter().enumerate() {
        let at = format!("$.method_runs.entropy.source_ablations[{i}]");
        exact_keys(ablation, &["removed_source_id", "metrics"], &at)?;
        let removed = text(ablation, "removed_source_id", &at)?.to_owned();
        if !selected_ids.contains(&removed) || ablation_map.contains_key(&removed) {
            return Err(format!("{at}: ablation must name one unique selected source"));
        }
        ablation_map.insert(removed, Metrics::read(field(ablation, "metrics", &at)?, &format!("{at}.metrics"))?);
    }
    let mut contributions = Vec::new();
    let mut negative_transfer = Vec::new();
    let full_metrics = dynamic_metrics;
    for source_id in selected_ids {
        let without = ablation_map.get(&source_id).ok_or_else(|| format!("missing ablation for {source_id}"))?;
        let delta = full_metrics.deltas_from(without);
        let negative = delta["negative_transfer_on_one_or_more_metrics"].as_bool().unwrap_or(false);
        if negative {
            negative_transfer.push(source_id.clone());
        }
        let weight = field(dynamic_run, "source_weights", "$.method_runs.entropy")?
            .as_array().ok_or_else(|| "source_weights: expected array".to_owned())?
            .iter()
            .find(|w| w["source_id"] == source_id)
            .and_then(|w| w["weight"].as_f64())
            .ok_or_else(|| format!("missing weight for {source_id}"))?;
        contributions.push(json!({
            "source_id": source_id,
            "logical_source_id": sources[&source_id].logical_source_id,
            "learned_weight": weight,
            "leave_one_source_out_delta": delta
        }));
    }
    let best_single = &method_map["BEST_SINGLE_SOURCE"].1;
    let comparisons = method_map.iter().map(|(method, (_, metrics))| {
        json!({
            "method_id": method,
            "metrics": metrics.to_json(),
            "delta_vs_best_single_source": metrics.differences_from(best_single)
        })
    }).collect::<Vec<_>>();
    let eligible_count = sources.values().filter(|s| s.eligible).count();
    let duplicate_logical_count = {
        let mut ids = BTreeSet::new();
        sources.values().filter(|s| !ids.insert(s.logical_source_id.clone())).count()
    };
    let _ = eligible_count;
    let _ = duplicate_logical_count;

    Ok(json!({
        "result": "PASS",
        "validator_version": "0.1.0",
        "benchmark_id": text(case, "benchmark_id", "$")?,
        "claim_scope": claim_scope,
        "evidence_class": evidence_class,
        "source_commit": source_commit,
        "case_sha256": case_sha,
        "target_split_sha256": target_split,
        "source_pool_count": sources.len(),
        "eligible_source_count": eligible_count,
        "excluded_sources": source_exclusions,
        "duplicate_logical_source_copy_count": duplicate_logical_count,
        "method_count": method_map.len(),
        "method_comparisons": comparisons,
        "source_contributions": contributions,
        "negative_transfer_source_ids": negative_transfer,
        "hard_gates": {
            "target_test_labels_sealed": true,
            "all_method_budgets_matched": true,
            "preprocessing_source_train_only": true,
            "source_weights_normalized": true,
            "ineligible_sources_zero_weight": true,
            "duplicate_sources_not_counted_independently": true,
            "source_ablation_complete": true
        },
        "claim_ceiling": text(case, "claim_ceiling", "$")?
    }))
}

pub fn evaluate_case(case: &Value, source_commit: &str, case_sha: &str) -> Check<Value> {
    evaluate(case, source_commit, case_sha)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../../benchmarks/maritime-source-mix/source-mix-case-v0.1.json");

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("source-mix fixture JSON")
    }
    fn evaluate(v: &Value) -> Check<Value> {
        evaluate_case(v, &"a".repeat(40), &"b".repeat(64))
    }

    #[test]
    fn frozen_source_mix_case_replays_and_finds_negative_transfer() {
        let out = evaluate(&fixture()).expect("valid synthetic source-mix case");
        assert_eq!(out["result"], "PASS");
        assert_eq!(out["method_count"], 4);
        assert_eq!(out["negative_transfer_source_ids"], json!(["src-simulated-adversarial"]));
        assert_eq!(out["source_contributions"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn excluded_sources_are_reported_without_becoming_positive_weight_sources() {
        let out = evaluate(&fixture()).unwrap();
        assert_eq!(out["source_pool_count"], 6);
        assert_eq!(out["eligible_source_count"], 3);
        assert_eq!(out["excluded_sources"].as_array().unwrap().len(), 3);
        assert_eq!(out["duplicate_logical_source_copy_count"], 1);
    }

    #[test]
    fn target_test_labels_may_not_select_source_weights() {
        let mut c = fixture();
        c["method_runs"][3]["target_test_labels_used"] = json!(true);
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn incompatible_taxonomy_source_cannot_receive_weight() {
        let mut c = fixture();
        c["method_runs"][3]["source_ids"][2] = json!("src-near-wrong-taxonomy");
        c["method_runs"][3]["source_weights"][2]["source_id"] = json!("src-near-wrong-taxonomy");
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn weak_label_source_cannot_receive_weight() {
        let mut c = fixture();
        c["method_runs"][3]["source_ids"][2] = json!("src-weak-labels");
        c["method_runs"][3]["source_weights"][2]["source_id"] = json!("src-weak-labels");
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn copied_source_is_not_an_independent_contributor() {
        let mut c = fixture();
        c["method_runs"][3]["source_ids"].as_array_mut().unwrap().push(json!("src-load25-copy"));
        c["method_runs"][3]["source_weights"].as_array_mut().unwrap().push(json!({"source_id":"src-load25-copy","weight":0.1}));
        c["method_runs"][3]["source_weights"][0]["weight"] = json!(0.45);
        c["method_runs"][3]["source_weights"][1]["weight"] = json!(0.25);
        c["method_runs"][3]["source_weights"][2]["weight"] = json!(0.2);
        c["method_runs"][3]["source_ablations"].as_array_mut().unwrap().push(json!({
            "removed_source_id":"src-load25-copy",
            "metrics":c["method_runs"][3]["metrics"].clone()
        }));
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn source_weights_must_sum_to_one() {
        let mut c = fixture();
        c["method_runs"][3]["source_weights"][0]["weight"] = json!(0.4);
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn uniform_fusion_must_use_equal_weights() {
        let mut c = fixture();
        c["method_runs"][2]["source_weights"][0]["weight"] = json!(0.5);
        c["method_runs"][2]["source_weights"][1]["weight"] = json!(0.25);
        c["method_runs"][2]["source_weights"][2]["weight"] = json!(0.25);
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn every_selected_source_requires_leave_one_out_ablation() {
        let mut c = fixture();
        c["method_runs"][3]["source_ablations"].as_array_mut().unwrap().pop();
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn target_split_and_compute_budget_must_be_matched() {
        let mut c = fixture();
        c["method_runs"][1]["compute_budget_id"] = json!("different-compute-budget");
        assert!(evaluate(&c).is_err());
        c = fixture();
        c["method_runs"][1]["target_split_sha256"] = json!("c".repeat(64));
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn cross_engine_claim_needs_independent_source_engines() {
        let mut c = fixture();
        c["claim_scope"] = json!("CROSS_ENGINE");
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn real_simulation_hybrid_claim_requires_both_source_roles() {
        let mut c = fixture();
        c["claim_scope"] = json!("REAL_SIMULATION_HYBRID");
        c["method_runs"][3]["source_ids"] = json!(["src-load25","src-load50"]);
        c["method_runs"][3]["source_weights"] = json!([
            {"source_id":"src-load25","weight":0.5},
            {"source_id":"src-load50","weight":0.5}
        ]);
        c["method_runs"][3]["source_ablations"] = json!([
            {"removed_source_id":"src-load25","metrics":c["method_runs"][3]["metrics"].clone()},
            {"removed_source_id":"src-load50","metrics":c["method_runs"][3]["metrics"].clone()}
        ]);
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn no_transfer_baseline_has_no_selected_source() {
        let mut c = fixture();
        c["method_runs"][0]["source_ids"] = json!(["src-load25"]);
        assert!(evaluate(&c).is_err());
    }

    #[test]
    fn posthoc_ablation_metrics_are_not_collapsed_into_a_single_score() {
        let out = evaluate(&fixture()).unwrap();
        let deltas = &out["source_contributions"][2]["leave_one_source_out_delta"];
        assert!(deltas.get("delta_macro_recall").is_some());
        assert!(deltas.get("delta_false_alarms_per_engine_hour").is_some());
        assert!(deltas.get("negative_transfer_on_one_or_more_metrics").is_some());
        assert!(out.get("composite_score").is_none());
    }
}

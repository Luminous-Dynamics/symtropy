// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Decision-specific forest forecast usefulness limits under regime change.
//!
//! Uses the shared forecast kernel for CRPS and nearest-rank intervals. It does not
//! estimate an empirical horizon from field data or provide a block-bootstrap guarantee.

use crate::forecast_kernel::{score_forecast_case, ForecastCase};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;

#[derive(Clone, Debug)]
struct Case {
    forecast: ForecastCase,
}

#[derive(Clone, Debug)]
struct BlockMetrics {
    block_id: String,
    eligible: usize,
    issued: usize,
    issuance_fraction: f64,
    crps_skill: Option<f64>,
    interval_coverage_80: Option<f64>,
    mean_absolute_median_error: Option<f64>,
    qualified: bool,
    failure_reasons: Vec<String>,
}

#[derive(Clone, Debug)]
struct HorizonMetrics {
    horizon_days: u32,
    qualified: bool,
    passing_block_count: usize,
    mean_issuance_fraction: f64,
    mean_crps_skill_score: Option<f64>,
    mean_interval_coverage_80: Option<f64>,
    mean_absolute_median_error: Option<f64>,
    blocks: Vec<BlockMetrics>,
}

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn number_at(value: &Value, key: &str) -> Result<f64, String> {
    let number = value.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key} must be numeric"))?;
    if !number.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    Ok(number)
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("String write cannot fail");
    }
    output
}

fn samples_at(value: &Value, key: &str, allow_empty: bool) -> Result<Vec<f64>, String> {
    let entries = value.get(key).and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array"))?;
    if entries.is_empty() && !allow_empty {
        return Err(format!("{key} must not be empty"));
    }
    let mut samples = Vec::with_capacity(entries.len());
    for entry in entries {
        let number = entry.as_f64().ok_or_else(|| format!("{key} entries must be numeric"))?;
        if !number.is_finite() {
            return Err(format!("{key} entries must be finite"));
        }
        samples.push(number);
    }
    Ok(samples)
}

fn option_horizon(value: &Value, key: &str) -> Result<Option<u32>, String> {
    match value.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(number) => {
            let raw = number.as_u64()
                .ok_or_else(|| format!("{key} must be a positive integer or null"))?;
            let parsed = u32::try_from(raw).map_err(|_| format!("{key} exceeds u32"))?;
            if parsed == 0 {
                return Err(format!("{key} must be positive"));
            }
            Ok(Some(parsed))
        }
    }
}

fn calculate_block(
    block_id: &str,
    cases: &[&Case],
    min_issuance: f64,
    coverage_min: f64,
    coverage_max: f64,
    decision_tolerance: f64,
) -> Result<BlockMetrics, String> {
    if cases.is_empty() {
        return Err(format!("block {block_id} has no eligible forecast cases"));
    }
    let eligible = cases.len();
    let mut issued = 0usize;
    let mut model_crps_sum = 0.0;
    let mut paired_baseline_crps_sum = 0.0;
    let mut median_error_sum = 0.0;
    let mut covered_80 = 0usize;

    for case in cases {
        let score = score_forecast_case(&case.forecast)
            .map_err(|error| format!("forecast {} scoring failed: {error}", case.forecast.forecast_id))?;
        if let Some(model_crps) = score.model_crps {
            issued += 1;
            model_crps_sum += model_crps;
            paired_baseline_crps_sum += score.baseline_crps;
            let median = score.median.ok_or_else(|| "issued forecast lacks median".to_string())?;
            let lower = score.lower_80.ok_or_else(|| "issued forecast lacks lower 80% bound".to_string())?;
            let upper = score.upper_80.ok_or_else(|| "issued forecast lacks upper 80% bound".to_string())?;
            median_error_sum += (median - case.forecast.observed).abs();
            if case.forecast.observed >= lower && case.forecast.observed <= upper {
                covered_80 += 1;
            }
        }
    }

    let issuance_fraction = issued as f64 / eligible as f64;
    let crps_skill = if issued > 0 {
        let paired_baseline_mean = paired_baseline_crps_sum / issued as f64;
        if paired_baseline_mean > 0.0 {
            Some(1.0 - (model_crps_sum / issued as f64) / paired_baseline_mean)
        } else {
            None
        }
    } else {
        None
    };
    let coverage = if issued > 0 {
        Some(covered_80 as f64 / issued as f64)
    } else {
        None
    };
    let mean_error = if issued > 0 {
        Some(median_error_sum / issued as f64)
    } else {
        None
    };

    let mut reasons = Vec::new();
    if issuance_fraction + 1e-12 < min_issuance {
        reasons.push("ISSUANCE_COVERAGE_BELOW_MINIMUM".to_string());
    }
    if crps_skill.map(|value| value <= 0.0).unwrap_or(true) {
        reasons.push("NON_POSITIVE_OR_UNDEFINED_PAIRED_CRPS_SKILL".to_string());
    }
    if coverage
        .map(|value| value < coverage_min || value > coverage_max)
        .unwrap_or(true)
    {
        reasons.push("80_PERCENT_INTERVAL_COVERAGE_OUTSIDE_BAND".to_string());
    }
    if mean_error.map(|value| value > decision_tolerance).unwrap_or(true) {
        reasons.push("DECISION_TOLERANCE_EXCEEDED".to_string());
    }

    Ok(BlockMetrics {
        block_id: block_id.to_string(),
        eligible,
        issued,
        issuance_fraction,
        crps_skill,
        interval_coverage_80: coverage,
        mean_absolute_median_error: mean_error,
        qualified: reasons.is_empty(),
        failure_reasons: reasons,
    })
}

fn validate_expected_context(actual: &Value, expected: &Value) -> Result<(), String> {
    if string_at(actual, "context_id")? != string_at(expected, "context_id")? {
        return Err("context ID oracle mismatch".to_string());
    }
    let actual_useful = option_horizon(actual, "useful_horizon_days")?;
    let expected_useful = option_horizon(expected, "useful_horizon_days")?;
    let actual_first = option_horizon(actual, "first_failure_horizon_days")?;
    let expected_first = option_horizon(expected, "first_failure_horizon_days")?;
    if actual_useful != expected_useful || actual_first != expected_first {
        return Err("forecast usefulness prefix / first-failure oracle mismatch".to_string());
    }
    if actual.get("later_horizon_recovery_after_failure").and_then(Value::as_bool)
        != expected.get("later_horizon_recovery_after_failure").and_then(Value::as_bool)
    {
        return Err("later-horizon recovery oracle mismatch".to_string());
    }
    let actual_rows = actual.get("horizon_results").and_then(Value::as_array)
        .ok_or_else(|| "actual horizon_results missing".to_string())?;
    let expected_rows = expected.get("horizon_results").and_then(Value::as_array)
        .ok_or_else(|| "expected horizon_results missing".to_string())?;
    if actual_rows.len() != expected_rows.len() {
        return Err("horizon result count oracle mismatch".to_string());
    }
    for (actual_row, expected_row) in actual_rows.iter().zip(expected_rows.iter()) {
        for key in ["horizon_days", "qualified", "passing_block_count"] {
            if actual_row.get(key) != expected_row.get(key) {
                return Err(format!("horizon oracle mismatch for {key}"));
            }
        }
        for key in [
            "mean_issuance_fraction",
            "mean_crps_skill_score",
            "mean_interval_coverage_80",
            "mean_absolute_median_error",
        ] {
            match (actual_row.get(key), expected_row.get(key)) {
                (Some(Value::Null), Some(Value::Null)) => {}
                (Some(actual_value), Some(expected_value)) => {
                    let actual_number = actual_value.as_f64()
                        .ok_or_else(|| format!("{key} actual must be numeric"))?;
                    let expected_number = expected_value.as_f64()
                        .ok_or_else(|| format!("{key} expected must be numeric"))?;
                    if (actual_number - expected_number).abs() > 1e-8 {
                        return Err(format!("horizon numeric oracle mismatch for {key}"));
                    }
                }
                _ => return Err(format!("horizon optional oracle mismatch for {key}")),
            }
        }
    }
    Ok(())
}

fn evaluate_context(context: &Value) -> Result<Value, String> {
    let context_id = string_at(context, "context_id")?;
    let variable = string_at(context, "variable")?;
    let unit = string_at(context, "unit")?;
    let regime_id = string_at(context, "regime_id")?;
    let baseline_id = string_at(context, "baseline_id")?;
    if context_id.trim().is_empty()
        || variable.trim().is_empty()
        || unit.trim().is_empty()
        || regime_id.trim().is_empty()
        || baseline_id.trim().is_empty()
    {
        return Err("context identity, variable, unit, regime and baseline must be non-empty".to_string());
    }

    let tolerance = number_at(context, "decision_tolerance")?;
    let min_issuance = number_at(context, "minimum_issuance_fraction")?;
    let coverage_min = number_at(context, "interval_80_coverage_min")?;
    let coverage_max = number_at(context, "interval_80_coverage_max")?;
    if tolerance <= 0.0
        || min_issuance <= 0.0
        || min_issuance > 1.0
        || coverage_min < 0.0
        || coverage_max > 1.0
        || coverage_min > coverage_max
    {
        return Err(format!("context {context_id} has invalid tolerance/coverage thresholds"));
    }

    let horizon_values = context.get("horizons_days").and_then(Value::as_array)
        .ok_or_else(|| "horizons_days must be an array".to_string())?;
    if horizon_values.is_empty() {
        return Err("horizons_days must not be empty".to_string());
    }
    let mut horizons = Vec::with_capacity(horizon_values.len());
    for value in horizon_values {
        let raw = value.as_u64().ok_or_else(|| "horizons must be positive integers".to_string())?;
        let horizon = u32::try_from(raw).map_err(|_| "horizon exceeds u32".to_string())?;
        if horizon == 0 || horizons.last().is_some_and(|previous| *previous >= horizon) {
            return Err("horizons must be positive and strictly increasing".to_string());
        }
        horizons.push(horizon);
    }

    let block_values = context.get("required_evaluation_blocks").and_then(Value::as_array)
        .ok_or_else(|| "required_evaluation_blocks must be an array".to_string())?;
    if block_values.len() < 2 {
        return Err("at least two evaluation blocks are required".to_string());
    }
    let mut block_ids = Vec::with_capacity(block_values.len());
    let mut unique_blocks = BTreeSet::new();
    for value in block_values {
        let block = value.as_str()
            .ok_or_else(|| "block IDs must be strings".to_string())?.to_string();
        if block.trim().is_empty() || !unique_blocks.insert(block.clone()) {
            return Err("block IDs must be non-empty and unique".to_string());
        }
        block_ids.push(block);
    }

    let cases_value = context.get("cases").and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_string())?;
    if cases_value.is_empty() {
        return Err("context cases must not be empty".to_string());
    }
    let valid_horizons = horizons.iter().copied().collect::<BTreeSet<_>>();
    let mut forecast_ids = BTreeSet::new();
    let mut grouped: BTreeMap<(u32, String), Vec<Case>> = BTreeMap::new();
    for (index, value) in cases_value.iter().enumerate() {
        let forecast_id = string_at(value, "forecast_id")?;
        let block_id = string_at(value, "block_id")?;
        let horizon_raw = value.get("horizon_days").and_then(Value::as_u64)
            .ok_or_else(|| format!("case {index} horizon_days must be positive integer"))?;
        let horizon = u32::try_from(horizon_raw).map_err(|_| "horizon exceeds u32".to_string())?;
        if !forecast_ids.insert(forecast_id.clone()) || forecast_id.trim().is_empty() {
            return Err("forecast IDs must be non-empty and unique within context".to_string());
        }
        if !unique_blocks.contains(&block_id) || !valid_horizons.contains(&horizon) {
            return Err(format!("forecast {forecast_id} has undeclared block or horizon"));
        }
        let observed = value.get("observed").and_then(Value::as_f64)
            .ok_or_else(|| "observed must be numeric".to_string())?;
        if !observed.is_finite() {
            return Err("observed must be finite".to_string());
        }
        let model_samples = samples_at(value, "model_samples", true)?;
        let baseline_samples = samples_at(value, "baseline_samples", false)?;
        let forecast = ForecastCase {
            forecast_id,
            variable: variable.clone(),
            unit: unit.clone(),
            horizon_days: horizon,
            observed,
            model_samples,
            baseline_samples,
        };
        grouped.entry((horizon, block_id)).or_default().push(Case { forecast });
    }

    let mut expected_case_count: Option<usize> = None;
    let mut horizon_results = Vec::new();
    for horizon in &horizons {
        let mut block_results = Vec::new();
        for block in &block_ids {
            let cases = grouped.get(&(*horizon, block.clone()))
                .ok_or_else(|| format!("missing declared support for horizon {horizon}, block {block}"))?;
            if let Some(expected) = expected_case_count {
                if cases.len() != expected {
                    return Err("case counts differ across block/horizon cells; support is incomplete".to_string());
                }
            } else {
                expected_case_count = Some(cases.len());
            }
            let refs = cases.iter().collect::<Vec<_>>();
            block_results.push(calculate_block(
                block, &refs, min_issuance, coverage_min, coverage_max, tolerance
            )?);
        }
        let passing = block_results.iter().filter(|block| block.qualified).count();
        let average_optional = |values: Vec<Option<f64>>| -> Option<f64> {
            if values.iter().any(Option::is_none) {
                None
            } else {
                Some(values.into_iter().flatten().sum::<f64>() / block_results.len() as f64)
            }
        };
        let mean_issuance = block_results.iter().map(|block| block.issuance_fraction).sum::<f64>()
            / block_results.len() as f64;
        let qualified = passing == block_results.len();
        horizon_results.push(HorizonMetrics {
            horizon_days: *horizon,
            qualified,
            passing_block_count: passing,
            mean_issuance_fraction: mean_issuance,
            mean_crps_skill_score: average_optional(block_results.iter().map(|b| b.crps_skill).collect()),
            mean_interval_coverage_80: average_optional(block_results.iter().map(|b| b.interval_coverage_80).collect()),
            mean_absolute_median_error: average_optional(block_results.iter().map(|b| b.mean_absolute_median_error).collect()),
            blocks: block_results,
        });
    }

    let mut useful_horizon = None;
    let mut first_failure = None;
    let mut failed = false;
    let mut later_recovery = false;
    for result in &horizon_results {
        if !failed && result.qualified {
            useful_horizon = Some(result.horizon_days);
        } else if !failed {
            failed = true;
            first_failure = Some(result.horizon_days);
        } else if result.qualified {
            later_recovery = true;
        }
    }

    let result_json = json!({
        "context_id":context_id,
        "variable":variable,
        "unit":unit,
        "regime_id":regime_id,
        "baseline_id":baseline_id,
        "decision_tolerance":tolerance,
        "useful_horizon_days":useful_horizon,
        "first_failure_horizon_days":first_failure,
        "later_horizon_recovery_after_failure":later_recovery,
        "horizon_results":horizon_results.iter().map(|result| json!({
            "horizon_days":result.horizon_days,
            "qualified":result.qualified,
            "passing_block_count":result.passing_block_count,
            "mean_issuance_fraction":result.mean_issuance_fraction,
            "mean_crps_skill_score":result.mean_crps_skill_score,
            "mean_interval_coverage_80":result.mean_interval_coverage_80,
            "mean_absolute_median_error":result.mean_absolute_median_error,
            "blocks":result.blocks.iter().map(|block| json!({
                "block_id":block.block_id,
                "eligible_case_count":block.eligible,
                "issued_case_count":block.issued,
                "issuance_fraction":block.issuance_fraction,
                "crps_skill_score":block.crps_skill,
                "interval_coverage_80":block.interval_coverage_80,
                "mean_absolute_median_error":block.mean_absolute_median_error,
                "qualified":block.qualified,
                "failure_reasons":block.failure_reasons
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    });
    let expected = context.get("expected")
        .ok_or_else(|| format!("context {context_id} expected oracle required"))?;
    validate_expected_context(&result_json, expected)?;
    Ok(result_json)
}

pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    if string_at(manifest, "schema_version")? != "0.1.0"
        || string_at(manifest, "benchmark_id")? != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || string_at(manifest, "corpus_id")? != "FOREST-FORECAST-USEFULNESS-LIMIT-001"
    {
        return Err("forest forecast usefulness corpus identity mismatch".to_string());
    }
    let ceiling = string_at(manifest, "claim_ceiling")?;
    if !ceiling.contains("All contexts, forecast ensembles")
        || !ceiling.contains("No empirical forest forecast usefulness limit")
    {
        return Err("forecast usefulness claim ceiling is incomplete".to_string());
    }
    let contexts = manifest.get("contexts").and_then(Value::as_array)
        .ok_or_else(|| "contexts must be an array".to_string())?;
    let expected_count = manifest.get("expected_context_count").and_then(Value::as_u64)
        .ok_or_else(|| "expected_context_count must be integer".to_string())? as usize;
    if contexts.len() != expected_count || contexts.is_empty() {
        return Err("forecast usefulness context count mismatch".to_string());
    }
    let mut context_ids = BTreeSet::new();
    let mut results = Vec::new();
    for context in contexts {
        let context_id = string_at(context, "context_id")?;
        if !context_ids.insert(context_id.clone()) {
            return Err("duplicate forecast usefulness context ID".to_string());
        }
        results.push(evaluate_context(context)?);
    }
    Ok(json!({
        "schema_version":"0.1.0",
        "benchmark_id":"LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id":"FOREST-FORECAST-USEFULNESS-LIMIT-001",
        "result":"PASS_SYNTHETIC_FORECAST_USEFULNESS_LIMIT",
        "source_manifest_sha256":sha256_hex(raw),
        "context_count":results.len(),
        "qualified_context_count":results.iter()
            .filter(|context| context["useful_horizon_days"].is_number()).count(),
        "context_results":results,
        "claim_ceiling":ceiling
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE:&str=include_str!("../../cases/forest-forecast-usefulness-synthetic-v0.1.json");

    #[test]
    fn forecast_usefulness_limit_reproduces_regime_specific_horizon_prefixes() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture parses");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        assert_eq!(receipt["result"],"PASS_SYNTHETIC_FORECAST_USEFULNESS_LIMIT");
        let results=receipt["context_results"].as_array().expect("contexts");
        assert_eq!(results[0]["useful_horizon_days"],90);
        assert_eq!(results[0]["first_failure_horizon_days"],180);
        assert_eq!(results[0]["later_horizon_recovery_after_failure"],true);
        assert_eq!(results[1]["useful_horizon_days"],30);
        assert_eq!(results[1]["first_failure_horizon_days"],90);
        assert_eq!(results[1]["later_horizon_recovery_after_failure"],true);
        assert_eq!(receipt["source_manifest_sha256"],sha256_hex(FIXTURE.as_bytes()));
    }

    #[test]
    fn later_good_horizon_does_not_reopen_a_failed_prefix() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        let stable=&receipt["context_results"][0];
        assert_eq!(stable["horizon_results"][2]["qualified"],false);
        assert_eq!(stable["horizon_results"][3]["qualified"],true);
        assert_eq!(stable["useful_horizon_days"],90);
    }

    #[test]
    fn forecast_abstention_remains_in_the_issuance_denominator() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        let post=&receipt["context_results"][1];
        assert_eq!(post["horizon_results"][2]["blocks"][0]["eligible_case_count"],4);
        assert_eq!(post["horizon_results"][2]["blocks"][0]["issued_case_count"],3);
        assert_eq!(post["horizon_results"][2]["qualified"],false);
        assert!(post["horizon_results"][2]["blocks"][0]["failure_reasons"]
            .as_array().expect("reasons").iter()
            .any(|reason|reason=="ISSUANCE_COVERAGE_BELOW_MINIMUM"));
    }

    #[test]
    fn duplicated_forecast_identity_is_rejected() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let duplicate=manifest["contexts"][0]["cases"][1]["forecast_id"].clone();
        manifest["contexts"][0]["cases"][0]["forecast_id"]=duplicate;
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn missing_block_horizon_support_is_rejected() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["contexts"][0]["cases"].as_array_mut().expect("cases")
            .retain(|case|case["block_id"]!="B4"||case["horizon_days"]!=180);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn non_positive_decision_tolerance_is_rejected() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["contexts"][0]["decision_tolerance"]=json!(0.0);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }
}

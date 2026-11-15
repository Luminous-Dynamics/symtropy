// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Exact analytic known-answer oracle for AFEX's correlated lognormal turnover ratio.
//!
//! This validates only the closed-form synthetic distribution. It is not a Monte Carlo
//! engine, an empirical AFEX uncertainty estimate, or a forest process model.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;

const TOLERANCE: f64 = 1.0e-9;
const DAYS_PER_YEAR: f64 = 365.25;

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn number_at(value: &Value, key: &str) -> Result<f64, String> {
    let number = value.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key} must be a number"))?;
    if !number.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    Ok(number)
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= TOLERANCE * expected.abs().max(1.0)
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[derive(Clone, Debug)]
struct RatioOracle {
    log_mean: f64,
    log_variance: f64,
    turnover_median: f64,
    turnover_mean: f64,
    turnover_standard_deviation: f64,
    turnover_lower: f64,
    turnover_upper: f64,
    lifespan_median_days: f64,
    lifespan_mean_days: f64,
    lifespan_standard_deviation_days: f64,
    lifespan_lower_days: f64,
    lifespan_upper_days: f64,
}

fn analytic_oracle(
    productivity_median: f64,
    stock_median: f64,
    productivity_log_sd: f64,
    stock_log_sd: f64,
    log_space_correlation: f64,
    z_lower: f64,
    z_upper: f64,
) -> Result<RatioOracle, String> {
    if productivity_median <= 0.0 || stock_median <= 0.0
        || productivity_log_sd <= 0.0 || stock_log_sd <= 0.0
        || !(-1.0..=1.0).contains(&log_space_correlation)
        || z_lower >= 0.0 || z_upper <= 0.0
    {
        return Err("invalid positive lognormal marginal or dependence parameters".to_string());
    }
    let log_mean = productivity_median.ln() - stock_median.ln();
    let log_variance = productivity_log_sd.powi(2) + stock_log_sd.powi(2)
        - 2.0 * log_space_correlation * productivity_log_sd * stock_log_sd;
    if !log_variance.is_finite() || log_variance < 0.0 {
        return Err("log-turnover variance is invalid".to_string());
    }
    let log_sd = log_variance.sqrt();
    let turnover_median = log_mean.exp();
    let turnover_mean = (log_mean + 0.5 * log_variance).exp();
    let turnover_standard_deviation =
        ((log_variance.exp() - 1.0) * (2.0 * log_mean + log_variance).exp()).sqrt();
    let turnover_lower = (log_mean + z_lower * log_sd).exp();
    let turnover_upper = (log_mean + z_upper * log_sd).exp();

    // Lifespan is the reciprocal ratio times the fixed 365.25-day conversion.
    let lifespan_log_mean = DAYS_PER_YEAR.ln() - log_mean;
    let lifespan_median_days = lifespan_log_mean.exp();
    let lifespan_mean_days = (lifespan_log_mean + 0.5 * log_variance).exp();
    let lifespan_standard_deviation_days =
        ((log_variance.exp() - 1.0)
            * (2.0 * lifespan_log_mean + log_variance).exp()).sqrt();
    let lifespan_lower_days = DAYS_PER_YEAR / turnover_upper;
    let lifespan_upper_days = DAYS_PER_YEAR / turnover_lower;

    let values = [
        log_mean, log_variance, turnover_median, turnover_mean, turnover_standard_deviation,
        turnover_lower, turnover_upper, lifespan_median_days, lifespan_mean_days,
        lifespan_standard_deviation_days, lifespan_lower_days, lifespan_upper_days,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err("analytic ratio result contains a non-finite value".to_string());
    }
    Ok(RatioOracle {
        log_mean, log_variance, turnover_median, turnover_mean,
        turnover_standard_deviation, turnover_lower, turnover_upper,
        lifespan_median_days, lifespan_mean_days, lifespan_standard_deviation_days,
        lifespan_lower_days, lifespan_upper_days,
    })
}

fn compare_field(
    checks: &mut Vec<Value>,
    id: &str,
    actual: f64,
    expected: f64,
) {
    let passed = close(actual, expected);
    checks.push(json!({
        "id": id,
        "passed": passed,
        "actual": actual,
        "expected": expected
    }));
}

pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    if string_at(manifest, "schema_version")? != "0.1.0"
        || string_at(manifest, "benchmark_id")? != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || string_at(manifest, "corpus_id")? != "AFEX-ROOT-DISTRIBUTION-UQ-001"
    {
        return Err("AFEX root-distribution corpus identity mismatch".to_string());
    }
    let claim_ceiling = string_at(manifest, "claim_ceiling")?;
    if !claim_ceiling.contains("not estimated from AFEX raw plot-level observations")
        || string_at(manifest, "evidence_class")? != "synthetic_analytic_known_truth" {
        return Err("AFEX root-distribution claim ceiling or evidence class invalid".to_string());
    }
    let execution = manifest.get("execution").ok_or_else(|| "execution metadata required".to_string())?;
    if execution.get("analytic_reference_calculated_independently").and_then(Value::as_bool) != Some(true)
        || execution.get("monte_carlo_engine_invoked").and_then(Value::as_bool) != Some(false)
        || execution.get("empirical_marginals_admitted").and_then(Value::as_bool) != Some(false)
        || execution.get("empirical_covariance_admitted").and_then(Value::as_bool) != Some(false)
        || execution.get("monte_carlo_engine_owner_issue").and_then(Value::as_u64) != Some(3973)
        || execution.get("first_order_covariance_engine_owner_issue").and_then(Value::as_u64) != Some(4652) {
        return Err("AFEX root-distribution execution boundary metadata invalid".to_string());
    }

    let model = manifest.get("measurement_model")
        .ok_or_else(|| "measurement_model required".to_string())?;
    let numerator = model.get("numerator").ok_or_else(|| "numerator required".to_string())?;
    let denominator = model.get("denominator").ok_or_else(|| "denominator required".to_string())?;
    let dependence = string_at(model, "dependence_model")?;
    let correlation_is_between = string_at(model, "correlation_is_between")?;
    if string_at(numerator, "distribution")? != "LOGNORMAL"
        || string_at(denominator, "distribution")? != "LOGNORMAL"
        || dependence != "GAUSSIAN_COPULA_EQUIVALENT_LOGSPACE_CORRELATION"
        || correlation_is_between != "log(productivity) and log(stock)"
        || string_at(model, "turnover")? != "productivity / stock"
        || string_at(model, "lifespan_days")? != "365.25 / turnover"
    {
        return Err("AFEX ratio measurement model semantics mismatch".to_string());
    }

    let p_median = number_at(numerator, "median_normalized")?;
    let s_median = number_at(denominator, "median_normalized")?;
    let p_log_sd = number_at(numerator, "log_standard_deviation")?;
    let s_log_sd = number_at(denominator, "log_standard_deviation")?;
    let z = manifest.get("z_quantiles").ok_or_else(|| "z_quantiles required".to_string())?;
    let z_lower = number_at(z, "central_95_lower")?;
    let z_upper = number_at(z, "central_95_upper")?;

    let cases = manifest.get("cases").and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_string())?;
    if cases.len() != 3 {
        return Err("exactly three dependence-ablation cases are required".to_string());
    }
    let expected_correlations = [0.6_f64, 0.0, -0.6];
    let mut seen_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(cases.len());
    let mut all_checks = Vec::new();

    for (index, case) in cases.iter().enumerate() {
        let scenario_id = string_at(case, "scenario_id")?;
        if scenario_id.trim().is_empty() || !seen_ids.insert(scenario_id.clone()) {
            return Err("scenario IDs must be non-empty and unique".to_string());
        }
        let rho = number_at(case, "log_space_correlation")?;
        if !close(rho, expected_correlations[index]) {
            return Err(format!("scenario {scenario_id} is out of frozen dependence order"));
        }
        let oracle = analytic_oracle(
            p_median, s_median, p_log_sd, s_log_sd, rho, z_lower, z_upper
        )?;
        let mut checks = Vec::new();
        compare_field(&mut checks, "LOG-TURNOVER-MEAN-001", number_at(case, "log_turnover_mean")?, oracle.log_mean);
        compare_field(&mut checks, "LOG-TURNOVER-VARIANCE-001", number_at(case, "log_turnover_variance")?, oracle.log_variance);

        let turnover = case.get("turnover").ok_or_else(|| "turnover summary required".to_string())?;
        compare_field(&mut checks, "TURNOVER-MEDIAN-001", number_at(turnover, "median")?, oracle.turnover_median);
        compare_field(&mut checks, "TURNOVER-MEAN-001", number_at(turnover, "mean")?, oracle.turnover_mean);
        compare_field(&mut checks, "TURNOVER-SD-001", number_at(turnover, "standard_deviation")?, oracle.turnover_standard_deviation);
        let tq = turnover.get("central_95_interval").and_then(Value::as_array)
            .ok_or_else(|| "turnover central_95_interval must be array".to_string())?;
        if tq.len() != 2 { return Err("turnover interval must have two endpoints".to_string()); }
        compare_field(&mut checks, "TURNOVER-Q025-001", tq[0].as_f64().ok_or_else(||"turnover lower quantile must be numeric".to_string())?, oracle.turnover_lower);
        compare_field(&mut checks, "TURNOVER-Q975-001", tq[1].as_f64().ok_or_else(||"turnover upper quantile must be numeric".to_string())?, oracle.turnover_upper);

        let lifespan = case.get("lifespan_days").ok_or_else(|| "lifespan summary required".to_string())?;
        compare_field(&mut checks, "LIFESPAN-MEDIAN-001", number_at(lifespan, "median")?, oracle.lifespan_median_days);
        compare_field(&mut checks, "LIFESPAN-MEAN-001", number_at(lifespan, "mean")?, oracle.lifespan_mean_days);
        compare_field(&mut checks, "LIFESPAN-SD-001", number_at(lifespan, "standard_deviation")?, oracle.lifespan_standard_deviation_days);
        let lq = lifespan.get("central_95_interval").and_then(Value::as_array)
            .ok_or_else(|| "lifespan central_95_interval must be array".to_string())?;
        if lq.len() != 2 { return Err("lifespan interval must have two endpoints".to_string()); }
        compare_field(&mut checks, "LIFESPAN-Q025-001", lq[0].as_f64().ok_or_else(||"lifespan lower quantile must be numeric".to_string())?, oracle.lifespan_lower_days);
        compare_field(&mut checks, "LIFESPAN-Q975-001", lq[1].as_f64().ok_or_else(||"lifespan upper quantile must be numeric".to_string())?, oracle.lifespan_upper_days);

        let passed = checks.iter().all(|check| check["passed"].as_bool() == Some(true));
        all_checks.extend(checks.clone());
        results.push(json!({
            "scenario_id": scenario_id,
            "log_space_correlation": rho,
            "analytic_log_turnover_variance": oracle.log_variance,
            "turnover": {
                "median": oracle.turnover_median,
                "mean": oracle.turnover_mean,
                "standard_deviation": oracle.turnover_standard_deviation,
                "central_95_interval": [oracle.turnover_lower, oracle.turnover_upper]
            },
            "lifespan_days": {
                "median": oracle.lifespan_median_days,
                "mean": oracle.lifespan_mean_days,
                "standard_deviation": oracle.lifespan_standard_deviation_days,
                "central_95_interval": [oracle.lifespan_lower_days, oracle.lifespan_upper_days]
            },
            "checks_passed": checks.iter().filter(|check| check["passed"].as_bool() == Some(true)).count(),
            "checks_total": checks.len(),
            "result": if passed { "PASS_ANALYTIC_KNOWN_ANSWER" } else { "FAIL" }
        }));
    }

    let passed_count = all_checks.iter().filter(|check| check["passed"].as_bool() == Some(true)).count();
    let total = all_checks.len();
    let passed = passed_count == total;
    Ok(json!({
        "schema_version": "0.1.0",
        "benchmark_id": "LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id": "AFEX-ROOT-DISTRIBUTION-UQ-001",
        "result": if passed { "PASS_SYNTHETIC_AFEX_ROOT_DISTRIBUTION_ORACLE" } else { "FAIL" },
        "source_manifest_sha256": sha256_hex(raw),
        "scenario_count": results.len(),
        "checks_total": total,
        "checks_passed": passed_count,
        "scenario_results": results,
        "monte_carlo_engine_invoked": false,
        "empirical_distribution_established": false,
        "empirical_afex_uncertainty": "NOT_ESTABLISHED",
        "claim_ceiling": claim_ceiling
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../cases/afex-root-ratio-distribution-analytic-v0.1.json");

    #[test]
    fn three_dependence_cases_match_closed_form_ratio_oracle() {
        let manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest).expect("analytic receipt");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_AFEX_ROOT_DISTRIBUTION_ORACLE");
        assert_eq!(receipt["scenario_count"], 3);
        assert_eq!(receipt["checks_passed"], 36);
        assert_eq!(receipt["checks_total"], 36);
        assert_eq!(receipt["monte_carlo_engine_invoked"], false);
        assert_eq!(receipt["empirical_afex_uncertainty"], "NOT_ESTABLISHED");
    }

    #[test]
    fn wrong_dependence_value_fails_known_answer_check() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        manifest["cases"][0]["turnover"]["central_95_interval"][0] = json!(0.99);
        let raw = serde_json::to_vec(&manifest).expect("serialize mutation");
        let receipt = validate_corpus(&raw, &manifest).expect("semantic failure receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn raw_afex_distribution_claim_is_rejected() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        manifest["execution"]["empirical_marginals_admitted"] = json!(true);
        let raw = serde_json::to_vec(&manifest).expect("serialize mutation");
        assert!(validate_corpus(&raw, &manifest).is_err());
    }

    #[test]
    fn monte_carlo_cannot_be_claimed_to_have_run() {
        let mut manifest: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        manifest["execution"]["monte_carlo_engine_invoked"] = json!(true);
        let raw = serde_json::to_vec(&manifest).expect("serialize mutation");
        assert!(validate_corpus(&raw, &manifest).is_err());
    }
}

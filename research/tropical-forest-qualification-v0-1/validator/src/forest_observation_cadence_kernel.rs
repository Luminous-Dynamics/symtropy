// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Latency-aware forest observation cadence and assimilation design.
//!
//! The kernel uses a transparent stationary scalar AR(1) process and exact
//! scalar-Gaussian variance updates. It is a synthetic cadence/design adapter,
//! not a calibrated forest state model or an ecological EVSI implementation.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;

const TOL: f64 = 5e-10;

#[derive(Clone, Debug)]
struct Profile {
    id: String,
    cadence_days: u64,
    latency_days: u64,
    measurement_variance: f64,
    effort_cost_per_sample: f64,
    disturbance_cost_per_sample: f64,
}

#[derive(Clone, Debug)]
struct ProfileSummary {
    id: String,
    available_count: u64,
    latest_age: Option<u64>,
    effort_cost: f64,
    disturbance_cost: f64,
    posterior_variance: f64,
}

#[derive(Clone, Debug)]
struct Portfolio {
    selected_ids: Vec<String>,
    measurement_count: u64,
    effort_cost: f64,
    disturbance_cost: f64,
    posterior_variance: f64,
    latest_age: Option<u64>,
}

#[derive(Clone, Debug)]
struct DeadlineResult {
    deadline_days: u64,
    profile_summaries: Vec<ProfileSummary>,
    feasible_portfolio_count: usize,
    best_portfolio: Portfolio,
    pareto_frontier: Vec<Portfolio>,
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

fn u64_at(value: &Value, key: &str) -> Result<u64, String> {
    value.get(key).and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("String write cannot fail");
    }
    output
}

fn round12(value: f64) -> f64 {
    (value * 1_000_000_000_000.0).round() / 1_000_000_000_000.0
}

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= TOL
}

fn parse_profiles(value: &Value) -> Result<Vec<Profile>, String> {
    let entries = value.as_array().ok_or_else(|| "measurement_profiles must be an array".to_string())?;
    if entries.is_empty() || entries.len() > 12 {
        return Err("measurement_profiles must contain 1..=12 profiles".to_string());
    }
    let mut ids = BTreeSet::new();
    let mut profiles = Vec::with_capacity(entries.len());
    for entry in entries {
        let profile = Profile {
            id: string_at(entry, "id")?,
            cadence_days: u64_at(entry, "cadence_days")?,
            latency_days: u64_at(entry, "latency_days")?,
            measurement_variance: number_at(entry, "measurement_variance")?,
            effort_cost_per_sample: number_at(entry, "effort_cost_per_sample")?,
            disturbance_cost_per_sample: number_at(entry, "disturbance_cost_per_sample")?,
        };
        if profile.id.trim().is_empty()
            || !ids.insert(profile.id.clone())
            || profile.cadence_days == 0
            || profile.measurement_variance <= 0.0
            || profile.effort_cost_per_sample < 0.0
            || profile.disturbance_cost_per_sample < 0.0
        {
            return Err(format!("profile {} has invalid identity, cadence, variance or cost", profile.id));
        }
        profiles.push(profile);
    }
    Ok(profiles)
}

fn sample_times(profile: &Profile, deadline: u64) -> Vec<u64> {
    let mut times = Vec::new();
    let mut time = 0u64;
    while time <= deadline {
        if time.saturating_add(profile.latency_days) <= deadline {
            times.push(time);
        }
        let Some(next) = time.checked_add(profile.cadence_days) else {
            break;
        };
        if next <= time {
            break;
        }
        time = next;
    }
    times
}

fn posterior_variance(phi: f64, deadline: u64, selected: &[&Profile]) -> Result<(f64, u64, Option<u64>), String> {
    let mut measurements = Vec::new();
    for profile in selected {
        for time in sample_times(profile, deadline) {
            measurements.push((time, profile.id.as_str(), profile.measurement_variance));
        }
    }
    measurements.sort_by(|left, right| {
        left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1))
    });

    let mut variance = 1.0f64;
    let mut last_time = 0u64;
    for (sample_time, _profile_id, observation_variance) in &measurements {
        if *sample_time > last_time {
            let delta = sample_time - last_time;
            let exponent = i32::try_from(delta.saturating_mul(2))
                .map_err(|_| "sampling interval is too large for AR(1) power".to_string())?;
            let decay = phi.powi(exponent);
            // Stationary variance is one, so process propagation is
            // P(t+delta) = phi^(2*delta) P(t) + (1 - phi^(2*delta)).
            variance = decay * variance + (1.0 - decay);
        }
        variance = variance * observation_variance / (variance + observation_variance);
        last_time = *sample_time;
    }

    if deadline > last_time {
        let delta = deadline - last_time;
        let exponent = i32::try_from(delta.saturating_mul(2))
            .map_err(|_| "decision horizon is too large for AR(1) power".to_string())?;
        let decay = phi.powi(exponent);
        variance = decay * variance + (1.0 - decay);
    }
    if !variance.is_finite() || variance < 0.0 || variance > 1.0 + TOL {
        return Err("posterior variance escaped stationary [0,1] bounds".to_string());
    }
    let count = u64::try_from(measurements.len())
        .map_err(|_| "measurement count exceeds u64".to_string())?;
    let latest_age = measurements.last().map(|(time, _, _)| deadline - *time);
    Ok((round12(variance), count, latest_age))
}

fn calculate_profile_summary(
    profile: &Profile,
    phi: f64,
    deadline: u64,
) -> Result<ProfileSummary, String> {
    let (variance, count, latest_age) = posterior_variance(phi, deadline, &[profile])?;
    let effort = count as f64 * profile.effort_cost_per_sample;
    let disturbance = count as f64 * profile.disturbance_cost_per_sample;
    Ok(ProfileSummary {
        id: profile.id.clone(),
        available_count: count,
        latest_age,
        effort_cost: round12(effort),
        disturbance_cost: round12(disturbance),
        posterior_variance: variance,
    })
}

fn calculate_portfolio(
    chosen: &[&Profile],
    phi: f64,
    deadline: u64,
) -> Result<Portfolio, String> {
    let (posterior, measurement_count, latest_age) =
        posterior_variance(phi, deadline, chosen)?;
    let effort = chosen.iter().map(|p| {
        sample_times(p, deadline).len() as f64 * p.effort_cost_per_sample
    }).sum::<f64>();
    let disturbance = chosen.iter().map(|p| {
        sample_times(p, deadline).len() as f64 * p.disturbance_cost_per_sample
    }).sum::<f64>();
    Ok(Portfolio {
        selected_ids: chosen.iter().map(|p| p.id.clone()).collect(),
        measurement_count,
        effort_cost: round12(effort),
        disturbance_cost: round12(disturbance),
        posterior_variance: posterior,
        latest_age,
    })
}

fn portfolio_order(left: &Portfolio, right: &Portfolio) -> Ordering {
    left.posterior_variance.total_cmp(&right.posterior_variance)
        .then_with(|| left.effort_cost.total_cmp(&right.effort_cost))
        .then_with(|| left.disturbance_cost.total_cmp(&right.disturbance_cost))
        .then_with(|| left.selected_ids.join("|").cmp(&right.selected_ids.join("|")))
}

fn frontier_order(left: &Portfolio, right: &Portfolio) -> Ordering {
    left.effort_cost.total_cmp(&right.effort_cost)
        .then_with(|| left.posterior_variance.total_cmp(&right.posterior_variance))
        .then_with(|| left.selected_ids.join("|").cmp(&right.selected_ids.join("|")))
}

fn pareto_frontier(portfolios: &[Portfolio]) -> Vec<Portfolio> {
    let mut result = portfolios.iter().filter(|candidate| {
        !portfolios.iter().any(|other| {
            other.effort_cost <= candidate.effort_cost + 1e-12
                && other.posterior_variance <= candidate.posterior_variance + 1e-12
                && (other.effort_cost < candidate.effort_cost - 1e-12
                    || other.posterior_variance < candidate.posterior_variance - 1e-12)
        })
    }).cloned().collect::<Vec<_>>();
    result.sort_by(frontier_order);
    result
}

fn evaluate_deadline(
    context: &Value,
    deadline_value: &Value,
    profiles: &[Profile],
    effort_budget: f64,
    disturbance_budget: f64,
) -> Result<DeadlineResult, String> {
    let deadline = u64_at(deadline_value, "decision_deadline_days")?;
    let phi = number_at(context, "phi")?;
    if !phi.is_finite() || phi <= 0.0 || phi >= 1.0 {
        return Err("AR(1) phi must lie strictly between zero and one".to_string());
    }

    let mut profile_summaries = Vec::with_capacity(profiles.len());
    let mut active_profiles = Vec::new();
    for profile in profiles {
        let summary = calculate_profile_summary(profile, phi, deadline)?;
        if summary.available_count > 0 {
            active_profiles.push(profile);
        }
        profile_summaries.push(summary);
    }

    let portfolio_count = 1usize.checked_shl(active_profiles.len() as u32)
        .ok_or_else(|| "portfolio enumeration overflow".to_string())?;
    let mut portfolios = Vec::new();
    for mask in 0..portfolio_count {
        let chosen = active_profiles.iter().enumerate()
            .filter_map(|(index, profile)| {
                if (mask & (1usize << index)) != 0 { Some(*profile) } else { None }
            })
            .collect::<Vec<_>>();
        let portfolio = calculate_portfolio(&chosen, phi, deadline)?;
        if portfolio.effort_cost <= effort_budget + 1e-12
            && portfolio.disturbance_cost <= disturbance_budget + 1e-12
        {
            portfolios.push(portfolio);
        }
    }
    if portfolios.is_empty() {
        return Err("no feasible observation portfolios".to_string());
    }

    let feasible_portfolio_count = portfolios.len();
    portfolios.sort_by(portfolio_order);
    let best_portfolio = portfolios[0].clone();
    let pareto = pareto_frontier(&portfolios);

    Ok(DeadlineResult {
        deadline_days: deadline,
        profile_summaries,
        feasible_portfolio_count,
        best_portfolio,
        pareto_frontier: pareto,
    })
}

fn compare_optional_u64(actual: Option<u64>, expected: &Value, key: &str) -> Result<(), String> {
    let parsed = match expected {
        Value::Null => None,
        Value::Number(number) => Some(number.as_u64()
            .ok_or_else(|| format!("{key} expected value must be non-negative integer or null"))?),
        _ => return Err(format!("{key} expected value must be integer or null")),
    };
    if actual != parsed {
        return Err(format!("{key} oracle mismatch: actual {actual:?}, expected {parsed:?}"));
    }
    Ok(())
}

fn compare_portfolio(actual: &Portfolio, expected: &Value, context: &str) -> Result<(), String> {
    let ids = expected.get("selected_profile_ids").and_then(Value::as_array)
        .ok_or_else(|| format!("{context} expected selected_profile_ids missing"))?;
    let ids = ids.iter().map(|item| item.as_str()
        .map(ToOwned::to_owned).ok_or_else(|| "portfolio profile IDs must be strings".to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    if actual.selected_ids != ids {
        return Err(format!("{context} selected profile IDs mismatch"));
    }
    if actual.measurement_count != u64_at(expected, "measurement_count")? {
        return Err(format!("{context} measurement_count mismatch"));
    }
    for (field, value) in [
        ("total_effort_cost_units", actual.effort_cost),
        ("total_ecological_disturbance_units", actual.disturbance_cost),
        ("posterior_variance", actual.posterior_variance),
    ] {
        if !close(value, number_at(expected, field)?) {
            return Err(format!("{context} {field} mismatch: actual {value}"));
        }
    }
    compare_optional_u64(actual.latest_age, expected.get("latest_observation_age_days")
        .ok_or_else(|| format!("{context} latest_observation_age_days missing"))?, "latest_observation_age_days")?;
    Ok(())
}

fn compare_deadline(actual: &DeadlineResult, expected: &Value, context_id: &str) -> Result<(), String> {
    if actual.deadline_days != u64_at(expected, "decision_deadline_days")? {
        return Err("decision deadline oracle mismatch".to_string());
    }
    if actual.feasible_portfolio_count != u64_at(expected, "feasible_portfolio_count")? as usize {
        return Err(format!("{context_id} feasible portfolio count mismatch at day {}", actual.deadline_days));
    }
    let expected_profiles = expected.get("profile_summaries").and_then(Value::as_array)
        .ok_or_else(|| "expected profile_summaries missing".to_string())?;
    if expected_profiles.len() != actual.profile_summaries.len() {
        return Err("profile summary count mismatch".to_string());
    }
    for (actual_profile, expected_profile) in actual.profile_summaries.iter().zip(expected_profiles.iter()) {
        if actual_profile.id != string_at(expected_profile, "profile_id")?
            || actual_profile.available_count != u64_at(expected_profile, "available_measurement_count")?
        {
            return Err("profile identity or available measurement count mismatch".to_string());
        }
        compare_optional_u64(actual_profile.latest_age,
            expected_profile.get("latest_observation_age_days")
                .ok_or_else(|| "profile latest_observation_age_days missing".to_string())?,
            "profile latest_observation_age_days")?;
        for (field, value) in [
            ("effort_cost_units", actual_profile.effort_cost),
            ("ecological_disturbance_units", actual_profile.disturbance_cost),
            ("posterior_variance", actual_profile.posterior_variance),
        ] {
            if !close(value, number_at(expected_profile, field)?) {
                return Err(format!("profile {} {field} oracle mismatch", actual_profile.id));
            }
        }
    }
    compare_portfolio(&actual.best_portfolio,
        expected.get("best_portfolio").ok_or_else(|| "expected best_portfolio missing".to_string())?,
        context_id)?;

    let expected_frontier = expected.get("pareto_frontier").and_then(Value::as_array)
        .ok_or_else(|| "expected pareto_frontier missing".to_string())?;
    if expected_frontier.len() != actual.pareto_frontier.len() {
        return Err(format!("{context_id} Pareto frontier size mismatch"));
    }
    for (index, (actual_portfolio, expected_portfolio)) in
        actual.pareto_frontier.iter().zip(expected_frontier.iter()).enumerate()
    {
        compare_portfolio(actual_portfolio, expected_portfolio,
            &format!("{context_id} Pareto portfolio {index}"))?;
    }
    Ok(())
}

fn deadline_json(result: &DeadlineResult) -> Value {
    json!({
        "decision_deadline_days": result.deadline_days,
        "profile_summaries": result.profile_summaries.iter().map(|profile| json!({
            "profile_id": profile.id,
            "available_measurement_count": profile.available_count,
            "latest_observation_age_days": profile.latest_age,
            "effort_cost_units": profile.effort_cost,
            "ecological_disturbance_units": profile.disturbance_cost,
            "posterior_variance": profile.posterior_variance
        })).collect::<Vec<_>>(),
        "feasible_portfolio_count": result.feasible_portfolio_count,
        "best_portfolio": portfolio_json(&result.best_portfolio),
        "pareto_frontier": result.pareto_frontier.iter().map(portfolio_json).collect::<Vec<_>>()
    })
}

fn portfolio_json(portfolio: &Portfolio) -> Value {
    json!({
        "selected_profile_ids": portfolio.selected_ids,
        "measurement_count": portfolio.measurement_count,
        "total_effort_cost_units": portfolio.effort_cost,
        "total_ecological_disturbance_units": portfolio.disturbance_cost,
        "posterior_variance": portfolio.posterior_variance,
        "latest_observation_age_days": portfolio.latest_age
    })
}

pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    if string_at(manifest, "schema_version")? != "0.1.0"
        || string_at(manifest, "benchmark_id")? != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || string_at(manifest, "corpus_id")? != "FOREST-OBSERVATION-CADENCE-001"
    {
        return Err("forest observation-cadence corpus identity mismatch".to_string());
    }
    let ceiling = string_at(manifest, "claim_ceiling")?;
    if !ceiling.contains("All AR(1) state parameters")
        || !ceiling.contains("No AFEX measurement cadence")
    {
        return Err("forest observation-cadence claim ceiling is incomplete".to_string());
    }
    if string_at(manifest, "time_unit")? != "discrete synthetic days" {
        return Err("forest observation-cadence time unit mismatch".to_string());
    }

    let state = manifest.get("state_process")
        .ok_or_else(|| "state_process required".to_string())?;
    let stationary_prior = number_at(state, "stationary_prior_variance")?;
    let prior_at_zero = number_at(state, "prior_variance_at_day_zero")?;
    if !close(stationary_prior, 1.0) || !close(prior_at_zero, 1.0) {
        return Err("synthetic AR(1) process requires stationary and initial variance one".to_string());
    }
    let budgets = manifest.get("budgets").ok_or_else(|| "budgets required".to_string())?;
    let effort_budget = number_at(budgets, "field_effort_units")?;
    let disturbance_budget = number_at(budgets, "ecological_disturbance_units")?;
    if effort_budget < 0.0 || disturbance_budget < 0.0 {
        return Err("observation-cadence budgets must be non-negative".to_string());
    }
    let profiles = parse_profiles(manifest.get("measurement_profiles")
        .ok_or_else(|| "measurement_profiles required".to_string())?)?;

    let contexts = manifest.get("contexts").and_then(Value::as_array)
        .ok_or_else(|| "contexts must be an array".to_string())?;
    if contexts.len() != u64_at(manifest, "expected_context_count")? as usize
        || contexts.len() != 2
    {
        return Err("observation-cadence context count mismatch".to_string());
    }
    let mut context_ids = BTreeSet::new();
    let mut results = Vec::new();
    let mut scenario_count = 0usize;
    for context in contexts {
        let context_id = string_at(context, "context_id")?;
        if !context_ids.insert(context_id.clone()) {
            return Err("duplicate observation-cadence context ID".to_string());
        }
        let phi = number_at(context, "phi")?;
        let q = number_at(context, "innovation_variance")?;
        if phi <= 0.0 || phi >= 1.0 || q <= 0.0 || !close(q, 1.0 - phi * phi) {
            return Err(format!("context {context_id} AR(1) process parameters invalid"));
        }
        let decisions = context.get("decision_results").and_then(Value::as_array)
            .ok_or_else(|| "decision_results must be an array".to_string())?;
        if decisions.len() != 3 {
            return Err(format!("context {context_id} must contain three decision deadlines"));
        }
        let mut deadline_results = Vec::new();
        let mut previous_deadline = None;
        for expected in decisions {
            let deadline = u64_at(expected, "decision_deadline_days")?;
            if previous_deadline.is_some_and(|previous| previous >= deadline) {
                return Err("decision deadlines must be strictly increasing".to_string());
            }
            previous_deadline = Some(deadline);
            let calculated = evaluate_deadline(context, expected, &profiles, effort_budget, disturbance_budget)?;
            compare_deadline(&calculated, expected, &context_id)?;
            deadline_results.push(deadline_json(&calculated));
            scenario_count += 1;
        }
        results.push(json!({
            "context_id": context_id,
            "phi": phi,
            "innovation_variance": q,
            "decision_results": deadline_results
        }));
    }

    Ok(json!({
        "schema_version":"0.1.0",
        "benchmark_id":"LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id":"FOREST-OBSERVATION-CADENCE-001",
        "result":"PASS_SYNTHETIC_FOREST_OBSERVATION_CADENCE",
        "source_manifest_sha256":sha256_hex(raw),
        "profile_count":profiles.len(),
        "context_count":results.len(),
        "decision_scenario_count":scenario_count,
        "process_parameter_semantics":"SYNTHETIC_STATIONARY_AR1_NOT_A_CALIBRATED_FOREST_PROCESS",
        "assay_cost_and_disturbance_semantics":"SYNTHETIC_RELATIVE_UNITS_NOT_FIELD_ESTIMATES",
        "context_results":results,
        "claim_ceiling":ceiling
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE:&str=include_str!("../../cases/forest-observation-cadence-synthetic-v0.1.json");

    #[test]
    fn six_cadence_scenarios_reproduce_profile_metrics_and_pareto_frontiers() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture parses");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        assert_eq!(receipt["result"],"PASS_SYNTHETIC_FOREST_OBSERVATION_CADENCE");
        assert_eq!(receipt["profile_count"],6);
        assert_eq!(receipt["context_count"],2);
        assert_eq!(receipt["decision_scenario_count"],6);
        assert_eq!(receipt["source_manifest_sha256"],sha256_hex(FIXTURE.as_bytes()));
    }

    #[test]
    fn a_ten_day_latency_produces_no_measurement_by_a_seven_day_deadline() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        let stable=&receipt["context_results"][0]["decision_results"][0];
        let profiles=stable["profile_summaries"].as_array().expect("profiles");
        let monthly=profiles.iter().find(|p|p["profile_id"]=="MONTHLY_LAB_10D_LATENCY").expect("monthly profile");
        assert_eq!(monthly["available_measurement_count"],0);
        assert_eq!(monthly["posterior_variance"],1.0);
        assert_eq!(monthly["latest_observation_age_days"],Value::Null);
    }

    #[test]
    fn measurement_variance_must_be_positive() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["measurement_profiles"][0]["measurement_variance"]=json!(0.0);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn process_noise_must_match_declared_stationary_ar1_process() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["contexts"][0]["innovation_variance"]=json!(0.2);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn increased_latency_cannot_increase_the_available_measurement_count() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["measurement_profiles"][1]["latency_days"]=json!(20);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn increasing_latency_cannot_add_observations_or_reduce_posterior_variance() {
        let fast = Profile {
            id: "FAST".to_string(),
            cadence_days: 3,
            latency_days: 2,
            measurement_variance: 0.25,
            effort_cost_per_sample: 0.05,
            disturbance_cost_per_sample: 0.002,
        };
        let delayed = Profile {
            latency_days: 20,
            ..fast.clone()
        };
        let fast_times = sample_times(&fast, 30);
        let delayed_times = sample_times(&delayed, 30);
        assert!(delayed_times.len() <= fast_times.len());
        let (fast_variance, fast_count, _) =
            posterior_variance(0.98, 30, &[&fast]).expect("fast schedule");
        let (delayed_variance, delayed_count, _) =
            posterior_variance(0.98, 30, &[&delayed]).expect("delayed schedule");
        assert!(delayed_count <= fast_count);
        assert!(delayed_variance + 1e-12 >= fast_variance);
    }

    #[test]
    fn no_measurement_available_by_deadline_preserves_prior_variance() {
        let delayed = Profile {
            id: "DELAYED".to_string(),
            cadence_days: 7,
            latency_days: 10,
            measurement_variance: 0.01,
            effort_cost_per_sample: 0.7,
            disturbance_cost_per_sample: 0.12,
        };
        let (variance, count, age) =
            posterior_variance(0.98, 7, &[&delayed]).expect("no eligible observation");
        assert_eq!(count, 0);
        assert_eq!(age, None);
        assert_eq!(variance, 1.0);
    }

    #[test]
    fn budgets_and_pareto_frontier_are_hard_gates() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["budgets"]["ecological_disturbance_units"]=json!(0.0);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }
}

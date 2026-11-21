// SPDX-License-Identifier: AGPL-3.0-or-later
//! Censoring-aware, competing-risk forecast evaluation for WPT asset lifecycles.
//!
//! v0 is deliberately a synthetic known-truth evaluator. It computes a
//! horizon-specific multicause IPCW Brier score under unconditional independent
//! right censoring, estimates the censoring survival function with a
//! reverse-Kaplan-Meier product-limit estimator on the evaluation cohort, and
//! reports an auxiliary all-cause Harrell C-index. It does not fit predictors,
//! claim conditional independent censoring, estimate calibration curves,
//! implement A-calibration / D-calibration, or issue field RUL claims.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const TARGET_EVENT_CODES: &[&str] = &[
    "STRUCTURAL_CRACK_LIMIT",
    "RUTTING_LIMIT",
    "INSULATION_RESISTANCE_LIMIT",
    "COIL_OPEN_CIRCUIT",
    "THERMAL_LIMIT",
    "SERVICE_AVAILABILITY_LIMIT",
    "OTHER",
];
const COMPETING_EVENT_CODES: &[&str] = &[
    "PLANNED_REPLACEMENT",
    "UNRELATED_ASSET_FAILURE",
    "ROAD_RECONSTRUCTION",
];
const SYNTHETIC_FLAG: &str = "SYNTHETIC_ONLY";
const PROBABILITY_SUM_TOLERANCE: f64 = 1e-9;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    let map = object(value, path)?;
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn number_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = field(value, key, path)?
        .as_f64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be numeric"))?;
    if !number.is_finite() {
        return Err(fail(&format!("{path}.{key}"), "must be finite"));
    }
    Ok(number)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn string_set(
    value: &Value,
    key: &str,
    path: &str,
    allowed: &[&str],
) -> Result<BTreeSet<String>, String> {
    let values = field(value, key, path)?
        .as_array()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an array"))?;
    if values.is_empty() {
        return Err(fail(&format!("{path}.{key}"), "must not be empty"));
    }
    let mut result = BTreeSet::new();
    for item in values {
        let item = item
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                fail(
                    &format!("{path}.{key}"),
                    "all items must be non-empty strings",
                )
            })?;
        if !allowed.contains(&item) {
            return Err(fail(
                &format!("{path}.{key}"),
                format!("unsupported event code {item}"),
            ));
        }
        if !result.insert(item.to_string()) {
            return Err(fail(
                &format!("{path}.{key}"),
                format!("duplicate event code {item}"),
            ));
        }
    }
    Ok(result)
}

#[derive(Debug, Clone)]
struct Subject {
    subject_id: String,
    cluster_id: String,
    observed_duration_hours: f64,
    outcome_kind: String,
    event_code: Option<String>,
    survival_probability: f64,
    cumulative_incidence: BTreeMap<String, f64>,
    cause_specific_hazard_rates: BTreeMap<String, Vec<f64>>,
}

impl Subject {
    fn has_terminal_event(&self) -> bool {
        self.outcome_kind != "RIGHT_CENSORED"
    }

    fn risk_score(&self) -> f64 {
        1.0 - self.survival_probability
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SurvivalEvaluation {
    pub run_id: String,
    pub horizon_hours: f64,
    pub subject_count: usize,
    pub eligible_subject_count: usize,
    pub censored_before_horizon_count: usize,
    pub distinct_cluster_count: usize,
    /// Multicause IPCW Brier score divided by the total evaluation-cohort size.
    /// None means that horizon failed the censoring-support gate.
    pub ipcw_competing_risk_brier_score: Option<f64>,
    /// Machine-readable reason when the horizon-specific IPCW Brier score is not evaluable.
    pub ipcw_brier_not_evaluable_reason: Option<String>,
    /// Cause-specific hazard log score; lower is better and censoring G is not estimated.
    pub cause_specific_hazard_log_score: f64,
    /// Empirical Aalen-Johansen event-free survival at the forecast horizon.
    /// This is an in-cohort descriptive reference, not an independent oracle.
    pub aalen_johansen_event_free_survival: f64,
    /// Empirical cause-specific cumulative incidence at the forecast horizon.
    pub aalen_johansen_cumulative_incidence: BTreeMap<String, f64>,
    pub aalen_johansen_event_count: usize,
    /// Smallest censoring-survival value used by an eligible score contribution.
    pub minimum_censoring_survival_used: Option<f64>,
    /// Largest inverse-probability-of-censoring weight used by an eligible subject.
    pub maximum_ipcw_weight: Option<f64>,
    /// Auxiliary ranking metric for any first terminal event, not a cause-specific C-index.
    pub all_cause_harrell_c_index: Option<f64>,
    pub all_cause_comparable_pairs: usize,
    pub censoring_estimator: &'static str,
    pub censoring_assumption: &'static str,
    pub claim_ceiling: &'static str,
}

fn parse_subjects(
    run: &Value,
    target_codes: &BTreeSet<String>,
    competing_codes: &BTreeSet<String>,
    all_codes: &BTreeSet<String>,
    interval_count: usize,
) -> Result<Vec<Subject>, String> {
    let rows = field(run, "subjects", "$")?
        .as_array()
        .ok_or_else(|| fail("$.subjects", "must be an array"))?;
    if rows.is_empty() {
        return Err(fail("$.subjects", "at least one subject is required"));
    }

    let mut subjects = Vec::with_capacity(rows.len());
    let mut subject_ids = BTreeSet::new();

    for (index, row) in rows.iter().enumerate() {
        let path = format!("$.subjects[{index}]");
        exact_keys(
            row,
            &[
                "subject_id",
                "cluster_id",
                "observed_duration_hours",
                "outcome_kind",
                "event_code",
                "predicted_survival_probability",
                "predicted_cumulative_incidence",
                "predicted_cause_specific_hazard_rates_per_hour",
            ],
            &path,
        )?;
        let subject_id = string_field(row, "subject_id", &path)?.to_string();
        if !subject_ids.insert(subject_id.clone()) {
            return Err(fail(&path, format!("duplicate subject_id {subject_id}")));
        }
        let cluster_id = string_field(row, "cluster_id", &path)?.to_string();
        let observed_duration_hours = number_field(row, "observed_duration_hours", &path)?;
        if observed_duration_hours <= 0.0 {
            return Err(fail(
                &path,
                "observed_duration_hours must be strictly positive",
            ));
        }

        let outcome_kind = string_field(row, "outcome_kind", &path)?.to_string();
        let event_value = field(row, "event_code", &path)?;
        let event_code = if event_value.is_null() {
            None
        } else {
            Some(
                event_value
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| fail(&format!("{path}.event_code"), "must be a string or null"))?
                    .to_string(),
            )
        };
        match outcome_kind.as_str() {
            "TARGET_FAILURE" => {
                let code = event_code
                    .as_ref()
                    .ok_or_else(|| fail(&path, "target failure requires an event_code"))?;
                if !target_codes.contains(code) {
                    return Err(fail(
                        &path,
                        format!("event code {code} is not in target_event_codes"),
                    ));
                }
            }
            "COMPETING_EVENT" => {
                let code = event_code
                    .as_ref()
                    .ok_or_else(|| fail(&path, "competing event requires an event_code"))?;
                if !competing_codes.contains(code) {
                    return Err(fail(
                        &path,
                        format!("event code {code} is not in competing_event_codes"),
                    ));
                }
            }
            "RIGHT_CENSORED" => {
                if event_code.is_some() {
                    return Err(fail(
                        &path,
                        "right-censored subject must not carry an event_code",
                    ));
                }
            }
            _ => return Err(fail(&path, "unknown outcome_kind")),
        }

        let survival_probability = number_field(row, "predicted_survival_probability", &path)?;
        if !(0.0..=1.0).contains(&survival_probability) {
            return Err(fail(
                &path,
                "predicted_survival_probability must be within [0, 1]",
            ));
        }

        let cause_rows = field(row, "predicted_cumulative_incidence", &path)?
            .as_array()
            .ok_or_else(|| {
                fail(
                    &format!("{path}.predicted_cumulative_incidence"),
                    "must be an array",
                )
            })?;
        if cause_rows.len() != all_codes.len() {
            return Err(fail(
                &path,
                "predicted_cumulative_incidence must contain every declared event cause exactly once",
            ));
        }
        let mut cumulative_incidence = BTreeMap::new();
        for (cause_index, cause_row) in cause_rows.iter().enumerate() {
            let cause_path = format!("{path}.predicted_cumulative_incidence[{cause_index}]");
            exact_keys(cause_row, &["event_code", "probability"], &cause_path)?;
            let cause = string_field(cause_row, "event_code", &cause_path)?.to_string();
            if !all_codes.contains(&cause) {
                return Err(fail(
                    &cause_path,
                    format!("event cause {cause} is not declared by this run"),
                ));
            }
            let probability = number_field(cause_row, "probability", &cause_path)?;
            if !(0.0..=1.0).contains(&probability) {
                return Err(fail(&cause_path, "probability must be within [0, 1]"));
            }
            if cumulative_incidence
                .insert(cause.clone(), probability)
                .is_some()
            {
                return Err(fail(&cause_path, format!("duplicate event cause {cause}")));
            }
        }
        if cumulative_incidence.len() != all_codes.len()
            || all_codes
                .iter()
                .any(|cause| !cumulative_incidence.contains_key(cause))
        {
            return Err(fail(
                &path,
                "predicted_cumulative_incidence is missing one or more declared event causes",
            ));
        }
        let probability_sum = survival_probability + cumulative_incidence.values().sum::<f64>();
        if (probability_sum - 1.0).abs() > PROBABILITY_SUM_TOLERANCE {
            return Err(fail(
                &path,
                format!(
                    "survival plus cumulative incidence probabilities must sum to 1 (got {probability_sum})"
                ),
            ));
        }

        let hazard_rows = field(row, "predicted_cause_specific_hazard_rates_per_hour", &path)?
            .as_array()
            .ok_or_else(|| {
                fail(
                    &format!("{path}.predicted_cause_specific_hazard_rates_per_hour"),
                    "must be an array",
                )
            })?;
        if hazard_rows.len() != all_codes.len() {
            return Err(fail(
                &path,
                "predicted_cause_specific_hazard_rates_per_hour must contain every declared event cause exactly once",
            ));
        }
        let mut cause_specific_hazard_rates = BTreeMap::new();
        for (hazard_index, hazard_row) in hazard_rows.iter().enumerate() {
            let hazard_path =
                format!("{path}.predicted_cause_specific_hazard_rates_per_hour[{hazard_index}]");
            exact_keys(
                hazard_row,
                &["event_code", "rates_per_interval"],
                &hazard_path,
            )?;
            let cause = string_field(hazard_row, "event_code", &hazard_path)?.to_string();
            if !all_codes.contains(&cause) {
                return Err(fail(
                    &hazard_path,
                    format!("hazard cause {cause} is not declared by this run"),
                ));
            }
            let rates = field(hazard_row, "rates_per_interval", &hazard_path)?
                .as_array()
                .ok_or_else(|| {
                    fail(
                        &format!("{hazard_path}.rates_per_interval"),
                        "must be an array",
                    )
                })?;
            if rates.len() != interval_count {
                return Err(fail(
                    &hazard_path,
                    format!(
                        "rates_per_interval must contain exactly {interval_count} interval rates"
                    ),
                ));
            }
            let mut parsed_rates = Vec::with_capacity(interval_count);
            for (rate_index, rate) in rates.iter().enumerate() {
                let rate = rate.as_f64().ok_or_else(|| {
                    fail(
                        &format!("{hazard_path}.rates_per_interval[{rate_index}]"),
                        "must be numeric",
                    )
                })?;
                if !rate.is_finite() || rate < 0.0 {
                    return Err(fail(
                        &format!("{hazard_path}.rates_per_interval[{rate_index}]"),
                        "hazard rate must be finite and non-negative",
                    ));
                }
                parsed_rates.push(rate);
            }
            if cause_specific_hazard_rates
                .insert(cause.clone(), parsed_rates)
                .is_some()
            {
                return Err(fail(
                    &hazard_path,
                    format!("duplicate hazard cause {cause}"),
                ));
            }
        }
        if cause_specific_hazard_rates.len() != all_codes.len()
            || all_codes
                .iter()
                .any(|cause| !cause_specific_hazard_rates.contains_key(cause))
        {
            return Err(fail(
                &path,
                "predicted_cause_specific_hazard_rates_per_hour is missing one or more declared event causes",
            ));
        }

        subjects.push(Subject {
            subject_id,
            cluster_id,
            observed_duration_hours,
            outcome_kind,
            event_code,
            survival_probability,
            cumulative_incidence,
            cause_specific_hazard_rates,
        });
    }

    Ok(subjects)
}

fn parse_hazard_time_edges(run: &Value) -> Result<Vec<f64>, String> {
    let values = field(run, "hazard_time_edges_hours", "$")?
        .as_array()
        .ok_or_else(|| fail("$.hazard_time_edges_hours", "must be an array"))?;
    if values.len() < 2 {
        return Err(fail(
            "$.hazard_time_edges_hours",
            "at least two interval edges are required",
        ));
    }
    let mut edges = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let edge = value.as_f64().ok_or_else(|| {
            fail(
                &format!("$.hazard_time_edges_hours[{index}]"),
                "must be numeric",
            )
        })?;
        if !edge.is_finite() || edge < 0.0 {
            return Err(fail(
                &format!("$.hazard_time_edges_hours[{index}]"),
                "edge must be finite and non-negative",
            ));
        }
        if edges.last().is_some_and(|previous| edge <= *previous) {
            return Err(fail(
                "$.hazard_time_edges_hours",
                "interval edges must be strictly increasing",
            ));
        }
        edges.push(edge);
    }
    if edges[0] != 0.0 {
        return Err(fail(
            "$.hazard_time_edges_hours",
            "the first hazard interval must start at hour zero",
        ));
    }
    Ok(edges)
}

/// Compute the cause-specific censored log score from piecewise-constant hazard
/// forecasts. The integrated total hazard is charged through each observed
/// event/censor time; an observed terminal event also receives -log of its
/// cause-specific hazard in the interval containing that event time.
///
/// The score does not estimate G(t) or use IPCW, but properness under random
/// censoring still relies on independent censoring. Scores are compared only
/// on the same declared censoring regime.
/// Reconstruct survival and cause-specific cumulative incidence implied by
/// piecewise-constant cause-specific hazard forecasts at one horizon.
fn implied_survival_and_cif(
    subject: &Subject,
    interval_edges: &[f64],
    horizon: f64,
) -> Result<(f64, BTreeMap<String, f64>), String> {
    if horizon > *interval_edges.last().expect("hazard intervals validated") {
        return Err("forecast horizon exceeds declared hazard interval coverage".into());
    }
    let mut survival = 1.0_f64;
    let mut cumulative_incidence = subject
        .cause_specific_hazard_rates
        .keys()
        .map(|cause| (cause.clone(), 0.0_f64))
        .collect::<BTreeMap<_, _>>();

    for (interval_index, edge_pair) in interval_edges.windows(2).enumerate() {
        let start = edge_pair[0];
        let end = edge_pair[1];
        if start >= horizon {
            break;
        }
        let exposure = (horizon.min(end) - start).max(0.0);
        if exposure == 0.0 {
            continue;
        }
        let total_rate = subject
            .cause_specific_hazard_rates
            .values()
            .map(|rates| rates[interval_index])
            .sum::<f64>();
        if !total_rate.is_finite() {
            return Err("total cause-specific hazard rate overflowed".into());
        }
        if total_rate > 0.0 {
            let interval_event_mass = 1.0 - (-total_rate * exposure).exp();
            for (cause, rates) in &subject.cause_specific_hazard_rates {
                let increment =
                    survival * (rates[interval_index] / total_rate) * interval_event_mass;
                let value = cumulative_incidence
                    .get_mut(cause)
                    .expect("cause keys were constructed from the hazard map");
                *value += increment;
                if !value.is_finite() {
                    return Err("forecast cumulative incidence integration overflowed".into());
                }
            }
            survival *= (-total_rate * exposure).exp();
            if !survival.is_finite() {
                return Err("forecast survival integration overflowed".into());
            }
        }
    }
    Ok((survival, cumulative_incidence))
}

/// Require the published survival/CIF values and piecewise hazard curves to
/// describe the same probability distribution at the declared horizon.
fn validate_forecast_representation_coherence(
    subjects: &[Subject],
    interval_edges: &[f64],
    horizon: f64,
) -> Result<(), String> {
    for subject in subjects {
        let (implied_survival, implied_cif) =
            implied_survival_and_cif(subject, interval_edges, horizon)?;
        if (implied_survival - subject.survival_probability).abs() > 1e-8 {
            return Err(format!(
                "subject {}: survival probability disagrees with integrated cause-specific hazards (declared {}, implied {})",
                subject.subject_id, subject.survival_probability, implied_survival
            ));
        }
        for (cause, declared) in &subject.cumulative_incidence {
            let implied = implied_cif.get(cause).copied().ok_or_else(|| {
                format!(
                    "subject {}: hazard forecast omits cause {cause}",
                    subject.subject_id
                )
            })?;
            if (declared - implied).abs() > 1e-8 {
                return Err(format!(
                    "subject {}: cumulative incidence for {cause} disagrees with integrated cause-specific hazards (declared {declared}, implied {implied})",
                    subject.subject_id
                ));
            }
        }
    }
    Ok(())
}

/// Empirical Aalen-Johansen event-free survival and cause-specific cumulative
/// incidence for the evaluation cohort. At tied times, all event/censor rows
/// remain in the pre-time risk set; event causes are updated before subjects
/// leave the risk set. Censoring affects later risk sets but does not count as
/// a failure. This in-cohort curve is descriptive, not held-out calibration.
fn aalen_johansen_at_horizon(
    subjects: &[Subject],
    all_codes: &BTreeSet<String>,
    horizon: f64,
) -> Result<(f64, BTreeMap<String, f64>, usize), String> {
    let mut cumulative_incidence = all_codes
        .iter()
        .map(|cause| (cause.clone(), 0.0_f64))
        .collect::<BTreeMap<_, _>>();
    let mut survival = 1.0_f64;
    let mut event_count = 0usize;

    let mut event_times = subjects
        .iter()
        .filter(|subject| {
            subject.has_terminal_event() && subject.observed_duration_hours <= horizon
        })
        .map(|subject| subject.observed_duration_hours)
        .collect::<Vec<_>>();
    event_times.sort_by(f64::total_cmp);
    event_times.dedup_by(|left, right| *left == *right);

    for event_time in event_times {
        let at_risk = subjects
            .iter()
            .filter(|subject| subject.observed_duration_hours >= event_time)
            .count();
        if at_risk == 0 {
            return Err("Aalen-Johansen encountered an empty risk set".into());
        }

        let mut events_by_cause = BTreeMap::<String, usize>::new();
        for subject in subjects.iter().filter(|subject| {
            subject.has_terminal_event() && subject.observed_duration_hours == event_time
        }) {
            let cause = subject
                .event_code
                .as_ref()
                .expect("validated terminal event must carry event code");
            *events_by_cause.entry(cause.clone()).or_default() += 1;
        }
        let total_events = events_by_cause.values().sum::<usize>();
        if total_events > at_risk {
            return Err("Aalen-Johansen event count exceeds the risk set".into());
        }
        for (cause, count) in events_by_cause {
            let increment = survival * count as f64 / at_risk as f64;
            let value = cumulative_incidence
                .get_mut(&cause)
                .ok_or_else(|| format!("Aalen-Johansen encountered undeclared cause {cause}"))?;
            *value += increment;
            event_count += count;
        }
        survival *= 1.0 - total_events as f64 / at_risk as f64;
    }
    Ok((survival, cumulative_incidence, event_count))
}

fn cause_specific_hazard_log_score(
    subjects: &[Subject],
    interval_edges: &[f64],
) -> Result<f64, String> {
    let interval_count = interval_edges.len() - 1;
    let mut total_score = 0.0;

    for subject in subjects {
        let mut integrated_total_hazard = 0.0;
        let mut event_hazard = None::<f64>;

        for (interval_index, edge_pair) in interval_edges.windows(2).enumerate() {
            let start = edge_pair[0];
            let end = edge_pair[1];
            if subject.observed_duration_hours < start {
                break;
            }
            let exposure_hours = (subject.observed_duration_hours.min(end) - start).max(0.0);
            if exposure_hours > 0.0 {
                let total_hazard_rate = subject
                    .cause_specific_hazard_rates
                    .values()
                    .map(|rates| rates[interval_index])
                    .sum::<f64>();
                integrated_total_hazard += total_hazard_rate * exposure_hours;
                if !integrated_total_hazard.is_finite() {
                    return Err("cause-specific integrated hazard overflowed".into());
                }
            }

            let contains_event_time = subject.has_terminal_event()
                && subject.observed_duration_hours >= start
                && (subject.observed_duration_hours < end
                    || (interval_index + 1 == interval_count
                        && subject.observed_duration_hours <= end));
            if contains_event_time {
                let cause = subject
                    .event_code
                    .as_ref()
                    .expect("validated terminal event has a cause");
                event_hazard = subject
                    .cause_specific_hazard_rates
                    .get(cause)
                    .map(|rates| rates[interval_index]);
                break;
            }
        }

        let mut subject_score = integrated_total_hazard;
        if subject.has_terminal_event() {
            let hazard = event_hazard.ok_or_else(|| {
                format!(
                    "subject {} event time is outside the declared hazard intervals",
                    subject.subject_id
                )
            })?;
            if hazard <= 0.0 {
                return Err(format!(
                    "NOT_EVALUABLE: subject {} has zero forecast hazard for observed cause {}",
                    subject.subject_id,
                    subject.event_code.as_deref().unwrap_or("UNKNOWN")
                ));
            }
            subject_score -= hazard.ln();
        }
        if !subject_score.is_finite() {
            return Err("cause-specific hazard log score is not finite".into());
        }
        total_score += subject_score;
        if !total_score.is_finite() {
            return Err("cause-specific hazard score accumulation overflowed".into());
        }
    }

    Ok(total_score / subjects.len() as f64)
}

/// Estimate P(C >= t) immediately before t using a reverse-Kaplan-Meier
/// product limit on right-censoring times in the evaluation cohort.
///
/// Terminal events are treated as observations of the censoring process at
/// their terminal-event time. Ties use the pre-time risk set, and only censor
/// times strictly less than t enter the left-limit product.
fn censoring_survival_left_limit(subjects: &[Subject], time: f64) -> f64 {
    let mut censor_times = subjects
        .iter()
        .filter(|subject| {
            subject.outcome_kind == "RIGHT_CENSORED" && subject.observed_duration_hours < time
        })
        .map(|subject| subject.observed_duration_hours)
        .collect::<Vec<_>>();
    censor_times.sort_by(f64::total_cmp);
    censor_times.dedup_by(|left, right| *left == *right);

    let mut survival = 1.0;
    for censor_time in censor_times {
        let at_risk = subjects
            .iter()
            .filter(|subject| subject.observed_duration_hours >= censor_time)
            .count();
        let censored_here = subjects
            .iter()
            .filter(|subject| {
                subject.outcome_kind == "RIGHT_CENSORED"
                    && subject.observed_duration_hours == censor_time
            })
            .count();
        if at_risk == 0 || censored_here > at_risk {
            return 0.0;
        }
        survival *= 1.0 - censored_here as f64 / at_risk as f64;
    }
    survival
}

fn competing_risk_multiclass_loss(
    subject: &Subject,
    event_code: Option<&str>,
    event_free_at_horizon: bool,
) -> f64 {
    let survival_target = if event_free_at_horizon { 1.0 } else { 0.0 };
    let mut loss = (subject.survival_probability - survival_target).powi(2);
    for (cause, prediction) in &subject.cumulative_incidence {
        let target = if event_code == Some(cause.as_str()) {
            1.0
        } else {
            0.0
        };
        loss += (prediction - target).powi(2);
    }
    loss
}

/// Evaluate one fixed-horizon competing-risk forecast.
///
/// The IPCW Brier score uses an unconditional reverse-KM censoring estimator
/// fitted to this evaluation cohort. It is only admitted when independent
/// right censoring is explicitly declared. Subjects censored before the
/// horizon are excluded; target and competing events before the horizon are
/// scored against their own cause-specific outcome category. The divisor is
/// the full test-cohort size, as in the standard IPCW Brier construction.
pub fn evaluate_run(run: &Value) -> Result<SurvivalEvaluation, String> {
    exact_keys(
        run,
        &[
            "schema_version",
            "run_id",
            "evidence_kind",
            "evaluation_split",
            "estimand",
            "censoring_assumption",
            "censoring_estimator",
            "prediction_horizon_hours",
            "minimum_censoring_survival",
            "hazard_time_edges_hours",
            "target_event_codes",
            "competing_event_codes",
            "prediction_model_sha256",
            "outcome_data_sha256",
            "subjects",
            "quality_flags",
        ],
        "$",
    )?;

    if string_field(run, "schema_version", "$")? != "wpt-rul-survival-eval-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let run_id = string_field(run, "run_id", "$")?.to_string();
    if string_field(run, "evidence_kind", "$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail(
            "$.evidence_kind",
            "v0 admits synthetic known-truth evaluation runs only",
        ));
    }
    if string_field(run, "evaluation_split", "$")? != "TEST" {
        return Err(fail(
            "$.evaluation_split",
            "v0 evaluates a frozen TEST cohort only",
        ));
    }
    if string_field(run, "estimand", "$")? != "COMPETING_RISK_CIF_AT_HORIZON" {
        return Err(fail("$.estimand", "unsupported survival estimand"));
    }
    if string_field(run, "censoring_assumption", "$")? != "INDEPENDENT_RIGHT_CENSORING" {
        return Err(fail(
            "$.censoring_assumption",
            "NOT_EVALUABLE: v0 requires independent right censoring; conditional/dependent censoring needs a declared conditional censoring model",
        ));
    }
    if string_field(run, "censoring_estimator", "$")? != "EVALUATION_COHORT_UNCONDITIONAL_KM" {
        return Err(fail(
            "$.censoring_estimator",
            "v0 supports evaluation-cohort unconditional Kaplan-Meier censoring estimation only",
        ));
    }

    let horizon = number_field(run, "prediction_horizon_hours", "$")?;
    if horizon <= 0.0 {
        return Err(fail(
            "$.prediction_horizon_hours",
            "must be strictly positive",
        ));
    }
    let minimum_g = number_field(run, "minimum_censoring_survival", "$")?;
    if !(0.0..=1.0).contains(&minimum_g) || minimum_g == 0.0 {
        return Err(fail(
            "$.minimum_censoring_survival",
            "must be within (0, 1]",
        ));
    }

    for digest_field in ["prediction_model_sha256", "outcome_data_sha256"] {
        let digest = string_field(run, digest_field, "$")?;
        if !valid_sha256(digest) {
            return Err(fail(
                &format!("$.{digest_field}"),
                "must be lowercase SHA-256 hex",
            ));
        }
    }

    let flags = field(run, "quality_flags", "$")?
        .as_array()
        .ok_or_else(|| fail("$.quality_flags", "must be an array"))?;
    if flags.len() != 1 || flags[0].as_str() != Some(SYNTHETIC_FLAG) {
        return Err(fail(
            "$.quality_flags",
            "v0 synthetic runs must carry exactly SYNTHETIC_ONLY",
        ));
    }

    let target_codes = string_set(run, "target_event_codes", "$", TARGET_EVENT_CODES)?;
    let competing_codes = string_set(run, "competing_event_codes", "$", COMPETING_EVENT_CODES)?;
    if !target_codes.is_disjoint(&competing_codes) {
        return Err(fail(
            "$",
            "target_event_codes and competing_event_codes must be disjoint",
        ));
    }
    let all_codes = target_codes
        .union(&competing_codes)
        .cloned()
        .collect::<BTreeSet<_>>();
    let hazard_time_edges = parse_hazard_time_edges(run)?;
    let interval_count = hazard_time_edges.len() - 1;
    let subjects = parse_subjects(
        run,
        &target_codes,
        &competing_codes,
        &all_codes,
        interval_count,
    )?;
    let maximum_observed_time = subjects
        .iter()
        .map(|subject| subject.observed_duration_hours)
        .fold(0.0_f64, f64::max);
    if maximum_observed_time > *hazard_time_edges.last().expect("at least two edges") {
        return Err(fail(
            "$.hazard_time_edges_hours",
            "last hazard interval must cover every observed event/censor time",
        ));
    }
    validate_forecast_representation_coherence(&subjects, &hazard_time_edges, horizon)?;
    let hazard_log_score = cause_specific_hazard_log_score(&subjects, &hazard_time_edges)?;
    let (aj_survival, aj_cif, aj_event_count) =
        aalen_johansen_at_horizon(&subjects, &all_codes, horizon)?;

    let mut brier_weighted_sum = 0.0;
    let eligible_count = subjects
        .iter()
        .filter(|subject| {
            subject.outcome_kind != "RIGHT_CENSORED" || subject.observed_duration_hours >= horizon
        })
        .count();
    let censored_before_horizon_count = subjects.len() - eligible_count;
    let mut minimum_g_used = 1.0_f64;
    let mut maximum_weight = 0.0_f64;
    let mut brier_not_evaluable_reason = None::<String>;

    for subject in &subjects {
        let has_event_by_horizon =
            subject.has_terminal_event() && subject.observed_duration_hours <= horizon;
        if subject.outcome_kind == "RIGHT_CENSORED" && subject.observed_duration_hours < horizon {
            continue;
        }

        let query_time = if has_event_by_horizon {
            subject.observed_duration_hours
        } else {
            horizon
        };
        let censoring_survival = censoring_survival_left_limit(&subjects, query_time);
        if !censoring_survival.is_finite() || censoring_survival < minimum_g {
            brier_not_evaluable_reason = Some(format!(
                "NOT_EVALUABLE: censoring positivity support failed at {} hours (G(t-)={censoring_survival}, required >= {minimum_g})",
                query_time
            ));
            break;
        }
        let weight = 1.0 / censoring_survival;
        if !weight.is_finite() {
            brier_not_evaluable_reason = Some("NOT_EVALUABLE: IPCW weight overflowed".into());
            break;
        }
        minimum_g_used = minimum_g_used.min(censoring_survival);
        maximum_weight = maximum_weight.max(weight);

        let observed_event_code = if has_event_by_horizon {
            subject.event_code.as_deref()
        } else {
            None
        };
        let event_free_at_horizon = !has_event_by_horizon;
        let loss =
            competing_risk_multiclass_loss(subject, observed_event_code, event_free_at_horizon);
        if !loss.is_finite() {
            brier_not_evaluable_reason =
                Some("NOT_EVALUABLE: competing-risk Brier loss overflowed".into());
            break;
        }
        brier_weighted_sum += weight * loss;
        if !brier_weighted_sum.is_finite() {
            brier_not_evaluable_reason =
                Some("NOT_EVALUABLE: IPCW Brier accumulation overflowed".into());
            break;
        }
    }

    if brier_not_evaluable_reason.is_none() && eligible_count == 0 {
        brier_not_evaluable_reason = Some(
            "NOT_EVALUABLE: no subjects have known event status at the declared horizon".into(),
        );
    }

    let brier_score = if brier_not_evaluable_reason.is_none() {
        let score = brier_weighted_sum / subjects.len() as f64;
        if score.is_finite() {
            Some(score)
        } else {
            brier_not_evaluable_reason =
                Some("NOT_EVALUABLE: IPCW Brier score is not finite".into());
            None
        }
    } else {
        None
    };

    let mut concordant = 0.0_f64;
    let mut comparable_pairs = 0usize;
    for (left_index, left) in subjects.iter().enumerate() {
        if !left.has_terminal_event() {
            continue;
        }
        for (right_index, right) in subjects.iter().enumerate() {
            if left_index == right_index
                || left.observed_duration_hours >= right.observed_duration_hours
            {
                continue;
            }
            comparable_pairs += 1;
            match left.risk_score().total_cmp(&right.risk_score()) {
                std::cmp::Ordering::Greater => concordant += 1.0,
                std::cmp::Ordering::Equal => concordant += 0.5,
                std::cmp::Ordering::Less => {}
            }
        }
    }
    let c_index = if comparable_pairs == 0 {
        None
    } else {
        Some(concordant / comparable_pairs as f64)
    };

    let distinct_cluster_count = subjects
        .iter()
        .map(|subject| subject.cluster_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();

    Ok(SurvivalEvaluation {
        run_id,
        horizon_hours: horizon,
        subject_count: subjects.len(),
        eligible_subject_count: eligible_count,
        censored_before_horizon_count,
        distinct_cluster_count,
        ipcw_competing_risk_brier_score: brier_score,
        ipcw_brier_not_evaluable_reason: brier_not_evaluable_reason,
        cause_specific_hazard_log_score: hazard_log_score,
        aalen_johansen_event_free_survival: aj_survival,
        aalen_johansen_cumulative_incidence: aj_cif,
        aalen_johansen_event_count: aj_event_count,
        minimum_censoring_survival_used: brier_score.map(|_| minimum_g_used),
        maximum_ipcw_weight: brier_score.map(|_| maximum_weight),
        all_cause_harrell_c_index: c_index,
        all_cause_comparable_pairs: comparable_pairs,
        censoring_estimator: "EVALUATION_COHORT_UNCONDITIONAL_KM",
        censoring_assumption: "INDEPENDENT_RIGHT_CENSORING",
        claim_ceiling: "Synthetic known-truth, fixed-horizon competing-risk scoring only. No field RUL accuracy, conditional-censoring validity, calibration test, safety or standards claim.",
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-rul-survival-v0/survival-prediction-run.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic survival fixture JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    #[test]
    fn synthetic_competing_risk_ipcw_score_matches_hand_calculation() {
        let report = evaluate_run(&fixture()).unwrap();
        assert_eq!(report.subject_count, 8);
        assert_eq!(report.eligible_subject_count, 6);
        assert_eq!(report.censored_before_horizon_count, 2);
        assert_eq!(report.distinct_cluster_count, 7);
        assert!(
            (report.ipcw_competing_risk_brier_score.unwrap() - 0.23453571428571426).abs() < 1e-12
        );
        assert!((report.minimum_censoring_survival_used.unwrap() - 0.7).abs() < 1e-12);
        assert!((report.maximum_ipcw_weight.unwrap() - 1.0 / 0.7).abs() < 1e-12);
        assert_eq!(report.ipcw_brier_not_evaluable_reason, None);
        assert_eq!(report.all_cause_comparable_pairs, 15);
        assert!((report.all_cause_harrell_c_index.unwrap() - 0.9).abs() < 1e-12);
    }

    #[test]
    fn hazard_forecast_must_reproduce_declared_survival_and_cif() {
        let mut run = fixture();
        run["subjects"][0]["predicted_cause_specific_hazard_rates_per_hour"][0]["rates_per_interval"]
            [0] = Value::from(0.5);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("survival probability disagrees with integrated cause-specific hazards")
        );
    }

    #[test]
    fn aalen_johansen_reference_matches_hand_calculated_competing_risk_curve() {
        let report = evaluate_run(&fixture()).unwrap();
        assert_eq!(report.aalen_johansen_event_count, 3);
        assert!((report.aalen_johansen_event_free_survival - 15.0 / 28.0).abs() < 1e-12);
        assert!(
            (report.aalen_johansen_cumulative_incidence["STRUCTURAL_CRACK_LIMIT"] - 1.0 / 7.0)
                .abs()
                < 1e-12
        );
        assert!(
            (report.aalen_johansen_cumulative_incidence["INSULATION_RESISTANCE_LIMIT"]
                - 5.0 / 28.0)
                .abs()
                < 1e-12
        );
        assert!(
            (report.aalen_johansen_cumulative_incidence["PLANNED_REPLACEMENT"] - 1.0 / 7.0).abs()
                < 1e-12
        );
        assert_eq!(
            report.aalen_johansen_cumulative_incidence["ROAD_RECONSTRUCTION"],
            0.0
        );
    }

    #[test]
    fn cause_specific_censored_log_score_matches_piecewise_hazard_calculation() {
        let report = evaluate_run(&fixture()).unwrap();
        assert!((report.cause_specific_hazard_log_score - 2.5669264097478806).abs() < 1e-12);
    }

    #[test]
    fn event_at_hazard_interval_boundary_uses_the_next_interval() {
        let baseline = evaluate_run(&fixture()).unwrap();
        let mut run = fixture();
        run["subjects"][0]["observed_duration_hours"] = Value::from(5.0);
        let boundary = evaluate_run(&run).unwrap();
        assert!(
            (boundary.cause_specific_hazard_log_score
                - baseline.cause_specific_hazard_log_score
                - 0.16094379124341002 / 8.0)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn rejects_non_monotonic_hazard_interval_edges() {
        let mut run = fixture();
        run["hazard_time_edges_hours"] = Value::Array(vec![
            Value::from(0.0),
            Value::from(10.0),
            Value::from(5.0),
            Value::from(20.0),
        ]);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("interval edges must be strictly increasing")
        );
    }

    #[test]
    fn rejects_zero_hazard_for_an_observed_event_cause() {
        let mut run = fixture();
        let aggregate_hazard = 0.10058986952713125 + 3.0 * 0.020117973905426252;
        let redistributed_hazard = aggregate_hazard / 3.0;
        for (index, hazard) in run["subjects"][0]["predicted_cause_specific_hazard_rates_per_hour"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            let rate = if index == 0 {
                0.0
            } else {
                redistributed_hazard
            };
            hazard["rates_per_interval"] = Value::Array(vec![Value::from(rate); 3]);
        }
        for (index, cause) in run["subjects"][0]["predicted_cumulative_incidence"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            cause["probability"] = Value::from(if index == 0 { 0.0 } else { 0.8 / 3.0 });
        }

        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("zero forecast hazard for observed cause")
        );
    }

    #[test]
    fn right_censored_before_horizon_is_not_relabelled_as_failure() {
        let report = evaluate_run(&fixture()).unwrap();
        assert_eq!(report.censored_before_horizon_count, 2);
        assert_eq!(report.eligible_subject_count, 6);
    }

    #[test]
    fn competing_event_is_scored_against_its_own_cause_category() {
        let mut run = fixture();
        let planned_replacement = run["subjects"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|subject| subject["subject_id"] == "asset-f")
            .unwrap();
        planned_replacement["predicted_survival_probability"] = Value::from(0.05);
        planned_replacement["predicted_cumulative_incidence"][0]["probability"] = Value::from(0.05);
        planned_replacement["predicted_cumulative_incidence"][1]["probability"] = Value::from(0.05);
        planned_replacement["predicted_cumulative_incidence"][2]["probability"] = Value::from(0.05);
        planned_replacement["predicted_cumulative_incidence"][3]["probability"] = Value::from(0.8);

        // Keep all three forecast representations coherent while making the
        // competing-event probability match the known observed cause.
        let total_hazard = -0.05_f64.ln() / 10.0;
        let low_cause_hazard = total_hazard / 19.0;
        for hazard in planned_replacement["predicted_cause_specific_hazard_rates_per_hour"]
            .as_array_mut()
            .unwrap()
        {
            let rate = if hazard["event_code"] == "PLANNED_REPLACEMENT" {
                low_cause_hazard * 16.0
            } else {
                low_cause_hazard
            };
            hazard["rates_per_interval"] = Value::Array(vec![Value::from(rate); 3]);
        }

        let changed = evaluate_run(&run).unwrap();
        let baseline = evaluate_run(&fixture()).unwrap();
        assert!(changed.ipcw_competing_risk_brier_score < baseline.ipcw_competing_risk_brier_score);
    }

    #[test]
    fn rejects_probability_vector_that_does_not_sum_to_one() {
        let mut run = fixture();
        run["subjects"][0]["predicted_survival_probability"] = Value::from(0.5);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("probabilities must sum to 1")
        );
    }

    #[test]
    fn rejects_missing_declared_cause_probability() {
        let mut run = fixture();
        run["subjects"][0]["predicted_cumulative_incidence"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("every declared event cause exactly once")
        );
    }

    #[test]
    fn rejects_duplicate_subject_identity() {
        let mut run = fixture();
        let first = run["subjects"][0].clone();
        run["subjects"].as_array_mut().unwrap().push(first);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("duplicate subject_id")
        );
    }

    #[test]
    fn rejects_dependent_censoring_when_only_unconditional_km_is_available() {
        let mut run = fixture();
        run["censoring_assumption"] = Value::String("COVARIATE_DEPENDENT_RIGHT_CENSORING".into());
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("NOT_EVALUABLE: v0 requires independent right censoring")
        );
    }

    #[test]
    fn reports_ipcw_horizon_as_not_evaluable_but_retains_hazard_score() {
        let mut run = fixture();
        run["minimum_censoring_survival"] = Value::from(0.8);
        let report = evaluate_run(&run).unwrap();
        assert_eq!(report.ipcw_competing_risk_brier_score, None);
        assert!(
            report
                .ipcw_brier_not_evaluable_reason
                .as_deref()
                .unwrap()
                .contains("NOT_EVALUABLE: censoring positivity support failed")
        );
        assert!(report.cause_specific_hazard_log_score.is_finite());
    }

    #[test]
    fn target_failure_cannot_use_a_competing_event_code() {
        let mut run = fixture();
        run["subjects"][0]["event_code"] = Value::String("ROAD_RECONSTRUCTION".into());
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("is not in target_event_codes")
        );
    }

    #[test]
    fn right_censored_subject_must_not_have_an_event_code() {
        let mut run = fixture();
        run["subjects"][2]["event_code"] = Value::String("STRUCTURAL_CRACK_LIMIT".into());
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("must not carry an event_code")
        );
    }

    #[test]
    fn all_censored_cohort_has_no_harrell_comparable_pairs() {
        let mut run = fixture();
        for subject in run["subjects"].as_array_mut().unwrap() {
            subject["outcome_kind"] = Value::String("RIGHT_CENSORED".into());
            subject["event_code"] = Value::Null;
        }
        // Keep censoring beyond the horizon so horizon status remains known.
        for subject in run["subjects"].as_array_mut().unwrap() {
            subject["observed_duration_hours"] = Value::from(20.0);
        }
        let report = evaluate_run(&run).unwrap();
        assert_eq!(report.all_cause_comparable_pairs, 0);
        assert_eq!(report.all_cause_harrell_c_index, None);
    }

    #[test]
    fn rejects_zero_or_negative_observation_duration() {
        let mut run = fixture();
        run["subjects"][0]["observed_duration_hours"] = Value::from(0.0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("strictly positive")
        );
    }

    #[test]
    fn rejects_unknown_top_level_properties() {
        let mut run = fixture();
        run["unreviewed_calibration"] = Value::from(0.99);
        assert!(evaluate_run(&run).unwrap_err().contains("unknown property"));
    }
}

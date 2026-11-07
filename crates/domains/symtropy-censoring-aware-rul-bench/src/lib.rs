//! Censoring-aware prognosis evaluation with competing failure causes.
//! Censoring survival is estimated from CALIBRATION episodes only. Preventive
//! replacement, repair, transfer and unknown early censoring are not silently
//! treated as independent administrative censoring.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const TOL: f64 = 1.0e-9;
const G_EPSILON: f64 = 1.0e-12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CensoringStep {
    pub exposure: f64,
    pub at_risk: usize,
    pub independent_censors: usize,
    pub survival_after: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrierScore {
    pub forecast_kind: String,
    pub cause_id: Option<String>,
    pub model_ref: String,
    pub model_digest_ref: String,
    pub horizon_exposure: f64,
    pub exposure_unit: String,
    pub prediction_count: usize,
    pub denominator_count: usize,
    pub event_observed_by_horizon: usize,
    pub ordinary_censored_before_horizon: usize,
    pub informative_censoring_excluded: usize,
    pub weighted_loss_sum: f64,
    pub brier_score: f64,
    pub minimum_censoring_survival_used: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegratedBrierScore {
    pub forecast_kind: String,
    pub cause_id: Option<String>,
    pub model_ref: String,
    pub model_digest_ref: String,
    pub lower_horizon: f64,
    pub upper_horizon: f64,
    pub exposure_unit: String,
    pub integrated_brier_score: Option<f64>,
    pub horizon_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub evidence_disposition: String,
    pub exposure_unit: String,
    pub calibration_episode_count: usize,
    pub evaluation_episode_count: usize,
    pub informative_calibration_censor_count_excluded: usize,
    pub censoring_curve: Vec<CensoringStep>,
    pub brier_scores: Vec<BrierScore>,
    pub integrated_brier_scores: Vec<IntegratedBrierScore>,
    pub limitations: Vec<String>,
}

fn push(errors: &mut Vec<Violation>, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(Violation { code: code.to_owned(), path: path.to_owned(), detail: detail.into() });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn is_independent_reason(reason: &str, policy: &Value) -> bool {
    policy["independent_censoring_reasons"].as_array().into_iter().flatten()
        .any(|value| value.as_str() == Some(reason))
}

fn is_informative_reason(reason: &str, policy: &Value) -> bool {
    policy["informative_censoring_reasons"].as_array().into_iter().flatten()
        .any(|value| value.as_str() == Some(reason))
}

fn group_value<'a>(episode: &'a Value, dimension: &str) -> Option<&'a str> {
    match dimension {
        "ASSET_ID" => text(episode, "asset_id"),
        "CAMPAIGN_REF" => text(episode, "campaign_ref"),
        "SITE_REF" => text(episode, "site_ref"),
        "MATERIAL_COHORT_REF" => text(episode, "material_cohort_ref"),
        "MACHINE_CONFIGURATION_REF" => text(episode, "machine_configuration_ref"),
        _ => None,
    }
}

/// Validate episode/censoring semantics, grouped partition separation and
/// forecast identity before scoring.
pub fn validate_case(case: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();
    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut errors, "UNSUPPORTED_SCHEMA_VERSION", "$.schema_version", "schema_version must be exactly 1");
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    let evidence_class = text(case, "evidence_class").unwrap_or("");
    if !matches!(evidence_class, "SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED") {
        push(&mut errors, "INVALID_EVIDENCE_CLASS", "$.evidence_class", "evidence class must be synthetic or measured");
    }

    let Some(profile) = case.get("exposure_profile") else {
        push(&mut errors, "MISSING_EXPOSURE_PROFILE", "$.exposure_profile", "exposure profile is required");
        return errors;
    };
    let Some(unit) = text(profile, "exposure_unit") else {
        push(&mut errors, "MISSING_EXPOSURE_UNIT", "$.exposure_profile.exposure_unit", "one explicit exposure denominator is required");
        return errors;
    };
    for key in ["definition_ref", "conversion_policy_ref"] {
        if text(profile, key).is_none() {
            push(&mut errors, "MISSING_EXPOSURE_PROVENANCE", &format!("$.exposure_profile.{key}"), "exposure definition and conversion policy are required");
        }
    }

    let Some(causes) = case.get("causes").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_CAUSES", "$.causes", "competing failure causes must be declared");
        return errors;
    };
    let mut cause_ids = BTreeSet::new();
    for (index, cause) in causes.iter().enumerate() {
        if let Some(id) = cause.as_str().filter(|s| !s.trim().is_empty()) {
            if !cause_ids.insert(id.to_owned()) {
                push(&mut errors, "DUPLICATE_CAUSE_ID", &format!("$.causes[{index}]"), "cause IDs must be unique");
            }
        } else {
            push(&mut errors, "INVALID_CAUSE_ID", &format!("$.causes[{index}]"), "cause IDs must be non-empty strings");
        }
    }
    if cause_ids.is_empty() {
        push(&mut errors, "EMPTY_CAUSES", "$.causes", "at least one failure cause is required");
    }

    let Some(policy) = case.get("censoring_policy") else {
        push(&mut errors, "MISSING_CENSORING_POLICY", "$.censoring_policy", "censoring policy is required");
        return errors;
    };
    if text(policy, "censoring_reference_partition") != Some("CALIBRATION") {
        push(&mut errors, "INVALID_CENSORING_REFERENCE_PARTITION", "$.censoring_policy.censoring_reference_partition", "censoring survival must be estimated from CALIBRATION episodes only");
    }
    for key in ["independence_assumption_ref", "censoring_model_ref"] {
        if text(policy, key).is_none() {
            push(&mut errors, "MISSING_CENSORING_PROVENANCE", &format!("$.censoring_policy.{key}"), "independence assumption and censoring model references are required");
        }
    }
    let independent: BTreeSet<&str> = policy["independent_censoring_reasons"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
    let informative: BTreeSet<&str> = policy["informative_censoring_reasons"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
    if independent.is_empty() || informative.is_empty() {
        push(&mut errors, "INCOMPLETE_CENSORING_REASON_CLASSES", "$.censoring_policy", "independent and informative censoring reason classes must both be declared");
    }
    if !independent.is_disjoint(&informative) {
        push(&mut errors, "OVERLAPPING_CENSORING_REASON_CLASSES", "$.censoring_policy", "a censoring reason cannot be independent and informative");
    }
    let allowed: BTreeSet<&str> = ["ADMINISTRATIVE", "LOSS_TO_FOLLOW_UP", "PREVENTIVE_REPLACEMENT", "REPAIR", "TRANSFER", "UNKNOWN"].into_iter().collect();
    if independent.union(&informative).any(|reason| !allowed.contains(reason)) {
        push(&mut errors, "UNKNOWN_CENSORING_REASON_CLASS", "$.censoring_policy", "unknown censoring reason class");
    }

    let Some(split) = case.get("split_policy") else {
        push(&mut errors, "MISSING_SPLIT_POLICY", "$.split_policy", "grouped holdout policy is required");
        return errors;
    };
    if split.get("no_group_overlap_across_partitions").and_then(Value::as_bool) != Some(true) {
        push(&mut errors, "GROUP_LEAKAGE_GUARD_DISABLED", "$.split_policy.no_group_overlap_across_partitions", "grouped partition separation must be enabled");
    }
    if text(split, "partition_profile_ref").is_none() {
        push(&mut errors, "MISSING_PARTITION_PROFILE", "$.split_policy.partition_profile_ref", "partition profile reference is required");
    }
    let dims_opt = split.get("held_out_group_dimensions").and_then(Value::as_array);
    if dims_opt.is_none_or(|dims| dims.is_empty()) {
        push(&mut errors, "MISSING_HOLDOUT_DIMENSIONS", "$.split_policy.held_out_group_dimensions", "at least one group holdout dimension is required");
    }
    let dimensions: Vec<&str> = dims_opt.into_iter().flatten().filter_map(Value::as_str).collect();
    for dimension in &dimensions {
        if !matches!(*dimension, "ASSET_ID" | "CAMPAIGN_REF" | "SITE_REF" | "MATERIAL_COHORT_REF" | "MACHINE_CONFIGURATION_REF") {
            push(&mut errors, "UNKNOWN_HOLDOUT_DIMENSION", "$.split_policy.held_out_group_dimensions", format!("unsupported dimension {dimension}"));
        }
    }
    if dimensions.iter().collect::<BTreeSet<_>>().len() != dimensions.len() {
        push(&mut errors, "DUPLICATE_HOLDOUT_DIMENSION", "$.split_policy.held_out_group_dimensions", "holdout dimensions must be unique");
    }

    let Some(episodes) = case.get("episodes").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_EPISODES", "$.episodes", "episodes must be an array");
        return errors;
    };
    if episodes.is_empty() {
        push(&mut errors, "EMPTY_EPISODES", "$.episodes", "at least one asset episode is required");
    }
    let mut episode_ids = BTreeSet::new();
    let mut asset_partitions: BTreeMap<String, String> = BTreeMap::new();
    let mut group_partitions: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut has_calibration = false;
    let mut has_evaluation = false;

    for (index, episode) in episodes.iter().enumerate() {
        let path = format!("$.episodes[{index}]");
        let Some(id) = text(episode, "episode_id") else {
            push(&mut errors, "MISSING_EPISODE_ID", &format!("{path}.episode_id"), "episode_id is required");
            continue;
        };
        if !episode_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_EPISODE_ID", &format!("{path}.episode_id"), "episode IDs must be unique");
        }
        let partition = text(episode, "partition").unwrap_or("");
        if !matches!(partition, "CALIBRATION" | "VALIDATION" | "PROSPECTIVE_HOLDOUT") {
            push(&mut errors, "INVALID_EPISODE_PARTITION", &format!("{path}.partition"), "episode partition is invalid");
        }
        has_calibration |= partition == "CALIBRATION";
        has_evaluation |= matches!(partition, "VALIDATION" | "PROSPECTIVE_HOLDOUT");

        let Some(asset_id) = text(episode, "asset_id") else {
            push(&mut errors, "MISSING_ASSET_ID", &format!("{path}.asset_id"), "asset_id is required");
            continue;
        };
        if let Some(previous) = asset_partitions.insert(asset_id.to_owned(), partition.to_owned()) {
            if previous != partition {
                push(&mut errors, "ASSET_EPISODE_PARTITION_LEAKAGE", &format!("{path}.asset_id"), format!("asset {asset_id} occurs in {previous} and {partition}"));
            }
        }
        for dimension in &dimensions {
            if let Some(value) = group_value(episode, dimension) {
                let key = ((*dimension).to_owned(), value.to_owned());
                if let Some(previous) = group_partitions.insert(key.clone(), partition.to_owned()) {
                    if previous != partition {
                        push(&mut errors, "GROUPED_PARTITION_LEAKAGE", &format!("{path}.{dimension}"), format!("group value {} occurs in {previous} and {partition}", key.1));
                    }
                }
            } else {
                push(&mut errors, "MISSING_GROUPED_HOLDOUT_VALUE", &format!("{path}.{dimension}"), format!("{dimension} is required"));
            }
        }
        for key in ["asset_family_ref", "asset_configuration_ref", "campaign_ref", "site_ref", "material_cohort_ref", "machine_configuration_ref", "source_ref", "adjudication_ref"] {
            if text(episode, key).is_none() {
                push(&mut errors, "MISSING_EPISODE_PROVENANCE", &format!("{path}.{key}"), "episode must identify asset, campaign, site, material, configuration, source and adjudication");
            }
        }
        if text(episode, "exposure_unit") != Some(unit) {
            push(&mut errors, "EXPOSURE_UNIT_MISMATCH", &format!("{path}.exposure_unit"), "unlike exposure denominators cannot be pooled in one case");
        }
        match (finite(episode, "entry_exposure"), finite(episode, "exit_exposure")) {
            (Some(entry), Some(exit)) if entry >= 0.0 && exit > entry => {}
            _ => push(&mut errors, "INVALID_EPISODE_EXPOSURE", &path, "require finite exit > entry >= 0"),
        }
        match text(episode, "outcome_type") {
            Some("FAILURE_EVENT") => {
                if text(episode, "cause_id").is_none_or(|cause| !cause_ids.contains(cause)) {
                    push(&mut errors, "UNKNOWN_FAILURE_CAUSE", &format!("{path}.cause_id"), "failure event requires a declared cause");
                }
                if !episode.get("censoring_reason").is_some_and(Value::is_null) {
                    push(&mut errors, "FAILURE_HAS_CENSOR_REASON", &format!("{path}.censoring_reason"), "failure event cannot have censoring reason");
                }
            }
            Some("RIGHT_CENSORED") => {
                if !episode.get("cause_id").is_some_and(Value::is_null) {
                    push(&mut errors, "CENSORED_EPISODE_HAS_CAUSE", &format!("{path}.cause_id"), "right-censored episode must have null cause");
                }
                if text(episode, "censoring_reason").is_none_or(|reason| !independent.contains(reason) && !informative.contains(reason)) {
                    push(&mut errors, "UNCLASSIFIED_CENSORING_REASON", &format!("{path}.censoring_reason"), "right-censored episode requires a classified reason");
                }
            }
            _ => push(&mut errors, "INVALID_EPISODE_OUTCOME", &format!("{path}.outcome_type"), "outcome must be FAILURE_EVENT or RIGHT_CENSORED"),
        }
        let Some(interventions) = episode.get("interventions").and_then(Value::as_array) else {
            push(&mut errors, "MISSING_INTERVENTION_HISTORY", &format!("{path}.interventions"), "interventions array is required, including when empty");
            continue;
        };
        let mut intervention_ids = BTreeSet::new();
        for (ii, intervention) in interventions.iter().enumerate() {
            let ipath = format!("{path}.interventions[{ii}]");
            let Some(intervention_id) = text(intervention, "intervention_id") else {
                push(&mut errors, "MISSING_INTERVENTION_ID", &format!("{ipath}.intervention_id"), "intervention ID is required");
                continue;
            };
            if !intervention_ids.insert(intervention_id.to_owned()) {
                push(&mut errors, "DUPLICATE_INTERVENTION_ID", &format!("{ipath}.intervention_id"), "intervention IDs must be unique within episode");
            }
            match finite(intervention, "exposure_at") {
                Some(time) if time >= finite(episode, "entry_exposure").unwrap_or(0.0)
                    && time <= finite(episode, "exit_exposure").unwrap_or(-1.0) => {}
                _ => push(&mut errors, "INTERVENTION_OUTSIDE_EPISODE", &format!("{ipath}.exposure_at"), "intervention time must lie within episode exposure"),
            }
            for key in ["intervention_type", "source_ref"] {
                if text(intervention, key).is_none() {
                    push(&mut errors, "INCOMPLETE_INTERVENTION_PROVENANCE", &format!("{ipath}.{key}"), "intervention type and source are required");
                }
            }
        }
    }
    if !has_calibration {
        push(&mut errors, "MISSING_CALIBRATION_COHORT", "$.episodes", "separate calibration cohort is required for censoring model");
    }
    if !has_evaluation {
        push(&mut errors, "MISSING_EVALUATION_COHORT", "$.episodes", "validation or prospective holdout cohort is required");
    }

    let Some(predictions) = case.get("predictions").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_PREDICTIONS", "$.predictions", "prediction rows are required");
        return errors;
    };
    let episode_map: BTreeMap<&str, &Value> = episodes.iter()
        .filter_map(|episode| text(episode, "episode_id").map(|id| (id, episode))).collect();
    let mut prediction_ids = BTreeSet::new();
    let mut prediction_keys = BTreeSet::new();
    let mut cif_sums: BTreeMap<(String, u64, String, String), f64> = BTreeMap::new();

    for (index, prediction) in predictions.iter().enumerate() {
        let path = format!("$.predictions[{index}]");
        let Some(id) = text(prediction, "prediction_id") else {
            push(&mut errors, "MISSING_PREDICTION_ID", &format!("{path}.prediction_id"), "prediction_id is required");
            continue;
        };
        if !prediction_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_PREDICTION_ID", &format!("{path}.prediction_id"), "prediction IDs must be unique");
        }
        let episode_id = text(prediction, "episode_id");
        let Some(episode) = episode_id.and_then(|key| episode_map.get(key).copied()) else {
            push(&mut errors, "UNKNOWN_PREDICTION_EPISODE", &format!("{path}.episode_id"), "prediction references an unknown episode");
            continue;
        };
        let partition = text(prediction, "partition").unwrap_or("");
        if partition != text(episode, "partition").unwrap_or("") || !matches!(partition, "VALIDATION" | "PROSPECTIVE_HOLDOUT") {
            push(&mut errors, "PREDICTION_PARTITION_MISMATCH", &format!("{path}.partition"), "predictions must match a validation/holdout episode partition");
        }
        let kind = text(prediction, "forecast_kind").unwrap_or("");
        let cause = text(prediction, "cause_id");
        match kind {
            "ALL_CAUSE_SURVIVAL" if cause.is_none() => {}
            "CAUSE_CUMULATIVE_INCIDENCE" if cause.is_some_and(|id| cause_ids.contains(id)) => {}
            "ALL_CAUSE_SURVIVAL" => push(&mut errors, "SURVIVAL_PREDICTION_HAS_CAUSE", &format!("{path}.cause_id"), "all-cause survival requires null cause_id"),
            "CAUSE_CUMULATIVE_INCIDENCE" => push(&mut errors, "CIF_PREDICTION_INVALID_CAUSE", &format!("{path}.cause_id"), "cause-specific incidence requires a declared cause"),
            _ => push(&mut errors, "INVALID_FORECAST_KIND", &format!("{path}.forecast_kind"), "invalid forecast kind"),
        }
        let horizon = finite(prediction, "horizon_exposure");
        if horizon.is_none_or(|value| value <= 0.0) {
            push(&mut errors, "INVALID_PREDICTION_HORIZON", &format!("{path}.horizon_exposure"), "horizon must be finite and positive");
        }
        let probability = finite(prediction, "probability");
        if probability.is_none_or(|value| !(0.0..=1.0).contains(&value)) {
            push(&mut errors, "INVALID_PREDICTION_PROBABILITY", &format!("{path}.probability"), "forecast probability must lie in [0,1]");
        }
        for key in ["model_ref", "model_digest_ref", "source_ref"] {
            if text(prediction, key).is_none() {
                push(&mut errors, "MISSING_PREDICTION_PROVENANCE", &format!("{path}.{key}"), "model reference, digest and source are required");
            }
        }
        if let (Some(episode_id), Some(horizon), Some(probability)) = (episode_id, horizon, probability) {
            let cause_key = cause.unwrap_or("");
            let key = (episode_id.to_owned(), horizon.to_bits(), kind.to_owned(), cause_key.to_owned());
            if !prediction_keys.insert(key) {
                push(&mut errors, "DUPLICATE_EPISODE_FORECAST", &path, "duplicate episode/horizon/kind/cause forecast");
            }
            if kind == "CAUSE_CUMULATIVE_INCIDENCE" {
                if let (Some(model), Some(digest)) = (text(prediction, "model_ref"), text(prediction, "model_digest_ref")) {
                    *cif_sums.entry((episode_id.to_owned(), horizon.to_bits(), model.to_owned(), digest.to_owned())).or_default() += probability;
                }
            }
        }
    }
    for ((episode_id, horizon, model, digest), sum) in cif_sums {
        if sum > 1.0 + TOL {
            push(&mut errors, "CAUSE_PROBABILITIES_EXCEED_ONE", "$.predictions", format!("CIF probabilities for {episode_id}/{}/{model}/{digest} sum to {sum}", f64::from_bits(horizon)));
        }
    }

    errors
}

fn censoring_curve(case: &Value) -> Result<Vec<CensoringStep>, Violation> {
    let episodes = case["episodes"].as_array().unwrap();
    let policy = &case["censoring_policy"];
    let calibration: Vec<&Value> = episodes.iter().filter(|episode| text(episode, "partition") == Some("CALIBRATION")).collect();
    let mut times: Vec<f64> = calibration.iter().filter(|episode| {
        text(episode, "outcome_type") == Some("RIGHT_CENSORED")
            && text(episode, "censoring_reason").is_some_and(|reason| is_independent_reason(reason, policy))
    }).filter_map(|episode| finite(episode, "exit_exposure")).collect();
    times.sort_by(f64::total_cmp);
    times.dedup_by(|a, b| (*a - *b).abs() <= TOL);

    let mut g = 1.0;
    let mut result = Vec::new();
    for time in times {
        let risk = calibration.iter().filter(|episode| {
            finite(episode, "entry_exposure").is_some_and(|entry| entry <= time)
                && finite(episode, "exit_exposure").is_some_and(|exit| exit >= time)
        }).count();
        let censors = calibration.iter().filter(|episode| {
            text(episode, "outcome_type") == Some("RIGHT_CENSORED")
                && text(episode, "censoring_reason").is_some_and(|reason| is_independent_reason(reason, policy))
                && finite(episode, "exit_exposure").is_some_and(|exit| (exit - time).abs() <= TOL)
        }).count();
        if risk == 0 || censors > risk {
            return Err(Violation {
                code: "INVALID_CENSORING_RISK_SET".to_owned(),
                path: "$.episodes".to_owned(),
                detail: format!("invalid censoring risk set at exposure {time}"),
            });
        }
        g *= 1.0 - censors as f64 / risk as f64;
        result.push(CensoringStep { exposure: time, at_risk: risk, independent_censors: censors, survival_after: g });
    }
    Ok(result)
}

/// Left-limit Kaplan–Meier estimate G(t-) for independent censoring.
fn g_left(curve: &[CensoringStep], time: f64) -> f64 {
    curve.iter().filter(|step| step.exposure < time)
        .fold(1.0, |g, step| g * (1.0 - step.independent_censors as f64 / step.at_risk as f64))
}

fn score_predictions(case: &Value, curve: &[CensoringStep]) -> Result<Vec<BrierScore>, Violation> {
    let episodes: BTreeMap<&str, &Value> = case["episodes"].as_array().unwrap().iter()
        .map(|episode| (text(episode, "episode_id").unwrap(), episode)).collect();
    let policy = &case["censoring_policy"];
    let mut groups: BTreeMap<(String, Option<String>, String, String, u64), Vec<&Value>> = BTreeMap::new();

    for prediction in case["predictions"].as_array().unwrap() {
        let key = (
            text(prediction, "forecast_kind").unwrap().to_owned(),
            text(prediction, "cause_id").map(str::to_owned),
            text(prediction, "model_ref").unwrap().to_owned(),
            text(prediction, "model_digest_ref").unwrap().to_owned(),
            finite(prediction, "horizon_exposure").unwrap().to_bits(),
        );
        groups.entry(key).or_default().push(prediction);
    }

    let unit = text(&case["exposure_profile"], "exposure_unit").unwrap().to_owned();
    let mut scores = Vec::new();
    for ((kind, cause, model, digest, horizon_bits), rows) in groups {
        let horizon = f64::from_bits(horizon_bits);
        let g_horizon = g_left(curve, horizon);
        let mut loss_sum = 0.0;
        let mut denominator = 0usize;
        let mut events = 0usize;
        let mut ordinary_censors = 0usize;
        let mut informative_excluded = 0usize;
        let mut minimum_g = 1.0f64;

        for prediction in &rows {
            let episode_id = text(prediction, "episode_id").unwrap();
            let episode = episodes[episode_id];
            let followup = finite(episode, "exit_exposure").unwrap() - finite(episode, "entry_exposure").unwrap();
            let predicted = finite(prediction, "probability").unwrap();

            let (target, weight) = if text(episode, "outcome_type") == Some("FAILURE_EVENT") && followup <= horizon + TOL {
                let g_event = g_left(curve, followup);
                if g_event <= G_EPSILON {
                    return Err(Violation {
                        code: "CENSORING_SUPPORT_VIOLATION".to_owned(),
                        path: "$.predictions".to_owned(),
                        detail: format!("G(t-) is zero at event exposure {followup} for {episode_id}"),
                    });
                }
                let target = if kind == "ALL_CAUSE_SURVIVAL" {
                    0.0
                } else if text(episode, "cause_id") == cause.as_deref() {
                    1.0
                } else {
                    0.0
                };
                events += 1;
                (target, 1.0 / g_event)
            } else if followup + TOL >= horizon {
                if g_horizon <= G_EPSILON {
                    return Err(Violation {
                        code: "CENSORING_SUPPORT_VIOLATION".to_owned(),
                        path: "$.predictions".to_owned(),
                        detail: format!("G(h-) is zero at horizon {horizon} for {episode_id}"),
                    });
                }
                let target = if kind == "ALL_CAUSE_SURVIVAL" { 1.0 } else { 0.0 };
                (target, 1.0 / g_horizon)
            } else {
                // No failure was observed before horizon, but follow-up ended early.
                let reason = text(episode, "censoring_reason").unwrap();
                if is_informative_reason(reason, policy) {
                    informative_excluded += 1;
                    continue;
                }
                if !is_independent_reason(reason, policy) {
                    return Err(Violation {
                        code: "UNCLASSIFIED_EARLY_CENSORING".to_owned(),
                        path: "$.episodes".to_owned(),
                        detail: format!("unclassified early censoring for {episode_id}"),
                    });
                }
                ordinary_censors += 1;
                // Standard IPCW Brier contributes zero for independently
                // censored-before-horizon subjects while retaining N in the
                // denominator. Informative censoring is removed from N instead.
                denominator += 1;
                continue;
            };

            let weighted_loss = weight * (target - predicted).powi(2);
            if !weighted_loss.is_finite() {
                return Err(Violation {
                    code: "NONFINITE_IPCW_LOSS".to_owned(),
                    path: "$.predictions".to_owned(),
                    detail: format!("non-finite weighted loss for {episode_id}"),
                });
            }
            loss_sum += weighted_loss;
            denominator += 1;
            minimum_g = minimum_g.min(1.0 / weight);
        }

        if denominator == 0 {
            return Err(Violation {
                code: "NO_ELIGIBLE_PREDICTIONS".to_owned(),
                path: "$.predictions".to_owned(),
                detail: format!("no eligible predictions for {kind}/{cause:?} at horizon {horizon}"),
            });
        }
        scores.push(BrierScore {
            forecast_kind: kind,
            cause_id: cause,
            model_ref: model,
            model_digest_ref: digest,
            horizon_exposure: horizon,
            exposure_unit: unit.clone(),
            prediction_count: rows.len(),
            denominator_count: denominator,
            event_observed_by_horizon: events,
            ordinary_censored_before_horizon: ordinary_censors,
            informative_censoring_excluded: informative_excluded,
            weighted_loss_sum: loss_sum,
            brier_score: loss_sum / denominator as f64,
            minimum_censoring_survival_used: minimum_g,
        });
    }

    scores.sort_by(|a, b| a.forecast_kind.cmp(&b.forecast_kind)
        .then(a.cause_id.cmp(&b.cause_id))
        .then(a.model_ref.cmp(&b.model_ref))
        .then(a.horizon_exposure.total_cmp(&b.horizon_exposure)));
    Ok(scores)
}

fn integrate_scores(scores: &[BrierScore]) -> Vec<IntegratedBrierScore> {
    let mut groups: BTreeMap<(String, Option<String>, String, String), Vec<&BrierScore>> = BTreeMap::new();
    for score in scores {
        groups.entry((score.forecast_kind.clone(), score.cause_id.clone(), score.model_ref.clone(), score.model_digest_ref.clone()))
            .or_default().push(score);
    }
    let mut result = Vec::new();
    for ((kind, cause, model, digest), mut group) in groups {
        group.sort_by(|a, b| a.horizon_exposure.total_cmp(&b.horizon_exposure));
        let lower = group.first().unwrap().horizon_exposure;
        let upper = group.last().unwrap().horizon_exposure;
        let ibs = if group.len() >= 2 && upper > lower {
            let mut area = 0.0;
            for pair in group.windows(2) {
                area += (pair[1].horizon_exposure - pair[0].horizon_exposure)
                    * (pair[0].brier_score + pair[1].brier_score) / 2.0;
            }
            Some(area / (upper - lower))
        } else {
            None
        };
        result.push(IntegratedBrierScore {
            forecast_kind: kind,
            cause_id: cause,
            model_ref: model,
            model_digest_ref: digest,
            lower_horizon: lower,
            upper_horizon: upper,
            exposure_unit: group[0].exposure_unit.clone(),
            integrated_brier_score: ibs,
            horizon_count: group.len(),
        });
    }
    result
}

/// Fit G(t-) from calibration episodes and score validation/holdout predictions.
pub fn evaluate_case(case: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_case(case);
    if !errors.is_empty() {
        return Err(errors);
    }
    let curve = censoring_curve(case).map_err(|error| vec![error])?;
    let scores = score_predictions(case, &curve).map_err(|error| vec![error])?;
    let integrated = integrate_scores(&scores);
    let episodes = case["episodes"].as_array().unwrap();
    let policy = &case["censoring_policy"];

    let calibration_count = episodes.iter().filter(|ep| text(ep, "partition") == Some("CALIBRATION")).count();
    let evaluation_count = episodes.iter().filter(|ep| matches!(text(ep, "partition"), Some("VALIDATION" | "PROSPECTIVE_HOLDOUT"))).count();
    let informative_censor_count = episodes.iter().filter(|ep| {
        text(ep, "partition") == Some("CALIBRATION")
            && text(ep, "outcome_type") == Some("RIGHT_CENSORED")
            && text(ep, "censoring_reason").is_some_and(|reason| is_informative_reason(reason, policy))
    }).count();

    Ok(EvaluationReport {
        case_id: text(case, "case_id").unwrap().to_owned(),
        evidence_disposition: if text(case, "evidence_class") == Some("SYNTHETIC_CONFORMANCE_FIXTURE") { "SYNTHETIC_ONLY" } else { "MEASURED_PREDICTION_EVIDENCE_REVIEW_REQUIRED" }.to_owned(),
        exposure_unit: text(&case["exposure_profile"], "exposure_unit").unwrap().to_owned(),
        calibration_episode_count: calibration_count,
        evaluation_episode_count: evaluation_count,
        informative_calibration_censor_count_excluded: informative_censor_count,
        censoring_curve: curve,
        brier_scores: scores,
        integrated_brier_scores: integrated,
        limitations: vec![
            "This crate scores supplied probability forecasts; it does not train prognosis models".to_owned(),
            "Censoring survival is estimated from the CALIBRATION partition only".to_owned(),
            "Preventive replacement, repair, transfer and unknown early censoring are excluded from the primary score".to_owned(),
            "Independent censoring for administrative loss/follow-up must be reviewed empirically".to_owned(),
            "v0.1 evaluates all-cause survival and cause-specific cumulative incidence; it does not authorize maintenance actions".to_owned(),
            "Predictions are anchored at episode entry; dynamic landmark predictions and time-varying covariates are future work".to_owned(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-censoring-aware-rul-case.v0_1.json"
        )).expect("fixture parses")
    }

    fn has(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    fn score<'a>(report: &'a EvaluationReport, kind: &str, cause: Option<&str>, horizon: f64) -> &'a BrierScore {
        report.brier_scores.iter().find(|item| {
            item.forecast_kind == kind && item.cause_id.as_deref() == cause
                && (item.horizon_exposure - horizon).abs() <= TOL
        }).unwrap()
    }

    #[test]
    fn synthetic_fixture_is_valid() {
        assert!(validate_case(&fixture()).is_empty());
    }

    #[test]
    fn report_counts_calibration_and_evaluation_episodes() {
        let report = evaluate_case(&fixture()).unwrap();
        assert_eq!(report.calibration_episode_count, 8);
        assert_eq!(report.evaluation_episode_count, 6);
        assert_eq!(report.informative_calibration_censor_count_excluded, 2);
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY");
    }

    #[test]
    fn censoring_curve_uses_calibration_risk_set() {
        let report = evaluate_case(&fixture()).unwrap();
        let step = report.censoring_curve.iter().find(|step| (step.exposure - 7.0).abs() <= TOL).unwrap();
        assert_eq!(step.at_risk, 5);
        assert_eq!(step.independent_censors, 1);
        assert!((step.survival_after - 0.8).abs() <= TOL);
    }

    #[test]
    fn validation_brier_scores_are_finite_and_ipcw_weighted() {
        let report = evaluate_case(&fixture()).unwrap();
        assert!(score(&report, "ALL_CAUSE_SURVIVAL", None, 5.0).brier_score.is_finite());
        let h8 = score(&report, "ALL_CAUSE_SURVIVAL", None, 8.0);
        assert!(h8.brier_score.is_finite());
        assert!((h8.minimum_censoring_survival_used - 0.8).abs() <= TOL);
    }

    #[test]
    fn all_cause_brier_and_integrated_score_match_golden_vectors() {
        let report = evaluate_case(&fixture()).unwrap();
        let h5 = score(&report, "ALL_CAUSE_SURVIVAL", None, 5.0);
        let h8 = score(&report, "ALL_CAUSE_SURVIVAL", None, 8.0);
        assert!((h5.brier_score - 0.0656).abs() <= TOL);
        assert!((h8.brier_score - 0.07696).abs() <= TOL);
        let ibs = report.integrated_brier_scores.iter()
            .find(|value| value.forecast_kind == "ALL_CAUSE_SURVIVAL").unwrap();
        assert!((ibs.integrated_brier_score.unwrap() - 0.07128).abs() <= TOL);
    }

    #[test]
    fn cause_specific_cumulative_incidence_is_scored_per_failure_cause() {
        let report = evaluate_case(&fixture()).unwrap();
        assert!(report.brier_scores.iter().any(|s| s.forecast_kind == "CAUSE_CUMULATIVE_INCIDENCE" && s.cause_id.as_deref() == Some("PICK_FRACTURE")));
        assert!(report.integrated_brier_scores.iter().any(|s| s.forecast_kind == "CAUSE_CUMULATIVE_INCIDENCE" && s.cause_id.as_deref() == Some("CUTTER_WEAR") && s.integrated_brier_score.is_some()));
    }

    #[test]
    fn informative_early_intervention_is_excluded_from_primary_score() {
        let report = evaluate_case(&fixture()).unwrap();
        let h8 = score(&report, "ALL_CAUSE_SURVIVAL", None, 8.0);
        assert_eq!(h8.informative_censoring_excluded, 1);
        assert_eq!(h8.denominator_count, 5);
    }

    #[test]
    fn ordinary_early_administrative_censoring_is_retained_with_zero_ipcw_loss() {
        let report = evaluate_case(&fixture()).unwrap();
        let h8 = score(&report, "ALL_CAUSE_SURVIVAL", None, 8.0);
        assert_eq!(h8.ordinary_censored_before_horizon, 1);
        assert_eq!(h8.denominator_count, 5);
    }

    #[test]
    fn integrated_brier_score_uses_trapezoidal_horizon_average() {
        let report = evaluate_case(&fixture()).unwrap();
        let ibs = report.integrated_brier_scores.iter().find(|s| s.forecast_kind == "ALL_CAUSE_SURVIVAL").unwrap();
        assert_eq!(ibs.horizon_count, 2);
        assert!(ibs.integrated_brier_score.unwrap().is_finite());
    }

    #[test]
    fn failure_event_requires_declared_cause() {
        let mut case = fixture();
        case["episodes"][0]["cause_id"] = serde_json::json!("UNKNOWN_CAUSE");
        assert!(has(&validate_case(&case), "UNKNOWN_FAILURE_CAUSE"));
    }

    #[test]
    fn censored_episode_must_not_have_failure_cause() {
        let mut case = fixture();
        case["episodes"][2]["cause_id"] = serde_json::json!("PICK_FRACTURE");
        assert!(has(&validate_case(&case), "CENSORED_EPISODE_HAS_CAUSE"));
    }

    #[test]
    fn failure_event_must_not_have_censor_reason() {
        let mut case = fixture();
        case["episodes"][0]["censoring_reason"] = serde_json::json!("ADMINISTRATIVE");
        assert!(has(&validate_case(&case), "FAILURE_HAS_CENSOR_REASON"));
    }

    #[test]
    fn exposure_denominators_cannot_be_mixed() {
        let mut case = fixture();
        case["episodes"][0]["exposure_unit"] = serde_json::json!("calendar-days");
        assert!(has(&validate_case(&case), "EXPOSURE_UNIT_MISMATCH"));
    }

    #[test]
    fn site_cannot_leak_across_partitions() {
        let mut case = fixture();
        let site = case["episodes"][0]["site_ref"].clone();
        case["episodes"][8]["site_ref"] = site;
        assert!(has(&validate_case(&case), "GROUPED_PARTITION_LEAKAGE"));
    }

    #[test]
    fn asset_cannot_leak_across_partitions() {
        let mut case = fixture();
        let asset = case["episodes"][0]["asset_id"].clone();
        case["episodes"][8]["asset_id"] = asset;
        assert!(has(&validate_case(&case), "ASSET_EPISODE_PARTITION_LEAKAGE"));
    }

    #[test]
    fn predictions_must_match_evaluation_partition() {
        let mut case = fixture();
        let mut row = case["predictions"][0].clone();
        row["episode_id"] = case["episodes"][0]["episode_id"].clone();
        row["partition"] = serde_json::json!("CALIBRATION");
        row["prediction_id"] = serde_json::json!("bad-calibration-prediction");
        case["predictions"].as_array_mut().unwrap().push(row);
        assert!(has(&validate_case(&case), "PREDICTION_PARTITION_MISMATCH"));
    }

    #[test]
    fn cause_probabilities_cannot_sum_above_one() {
        let mut case = fixture();
        let episode_id = case["predictions"][1]["episode_id"].as_str().unwrap().to_owned();
        let predictions = case["predictions"].as_array_mut().unwrap();
        for prediction in predictions.iter_mut() {
            if prediction["episode_id"] == episode_id
                && prediction["horizon_exposure"] == 5.0
                && prediction["forecast_kind"] == "CAUSE_CUMULATIVE_INCIDENCE"
            {
                prediction["probability"] = serde_json::json!(0.8);
            }
        }
        assert!(has(&validate_case(&case), "CAUSE_PROBABILITIES_EXCEED_ONE"));
    }

    #[test]
    fn zero_censoring_survival_support_is_a_hard_error() {
        let mut case = fixture();
        let episode_id = case["episodes"][9]["episode_id"].as_str().unwrap().to_owned();
        case["episodes"][9]["exit_exposure"] = serde_json::json!(13.0);
        let mut prediction = case["predictions"][0].clone();
        prediction["prediction_id"] = serde_json::json!("pred-outside-support");
        prediction["episode_id"] = serde_json::json!(episode_id);
        prediction["horizon_exposure"] = serde_json::json!(12.0);
        prediction["source_ref"] = serde_json::json!("artifact:outside-support");
        case["predictions"].as_array_mut().unwrap().push(prediction);
        assert!(evaluate_case(&case).is_err());
    }

    #[test]
    fn deterministic_replay() {
        let case = fixture();
        assert_eq!(evaluate_case(&case).unwrap(), evaluate_case(&case).unwrap());
    }
}

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const VERSION: &str = "0.1.0";
const CASE_IDS: [&str; 6] = [
    "CAPABILITY-GAP-001",
    "CIRCULAR-FORECAST-002",
    "DYNAMIC-CHANGE-003",
    "REPRESENTATION-ACCESS-004",
    "STALE-INCOMPLETE-005",
    "OUTCOME-LEAKAGE-006",
];
const STATUSES: [&str; 5] = ["verified", "reported", "unknown", "stale", "conflicted"];
const SOURCE_KINDS: [&str; 6] = [
    "direct_measurement",
    "site_self_report",
    "registry_status",
    "historical_outcome",
    "derived_estimate",
    "post_selection_outcome",
];

type Check<T> = Result<T, String>;

fn object<'a>(value: &'a Value, at: &str) -> Check<&'a serde_json::Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{at}: expected object"))
}

fn array<'a>(value: &'a Value, at: &str) -> Check<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| format!("{at}: expected array"))
}

fn field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(value, at)?
        .get(key)
        .ok_or_else(|| format!("{at}: missing field {key}"))
}

fn text<'a>(value: &'a Value, at: &str) -> Check<&'a str> {
    value
        .as_str()
        .ok_or_else(|| format!("{at}: expected string"))
}

fn text_field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    text(field(value, key, at)?, &format!("{at}.{key}"))
}

fn require_keys(value: &Value, required: &[&str], allowed: &[&str], at: &str) -> Check<()> {
    let map = object(value, at)?;
    let allowed_set: BTreeSet<&str> = allowed.iter().copied().collect();
    for key in required {
        if !map.contains_key(*key) {
            return Err(format!("{at}: missing required key {key}"));
        }
    }
    for key in map.keys() {
        if !allowed_set.contains(key.as_str()) {
            return Err(format!("{at}: unknown key {key}"));
        }
    }
    Ok(())
}

fn strings(value: &Value, at: &str) -> Check<Vec<String>> {
    array(value, at)?
        .iter()
        .enumerate()
        .map(|(i, item)| text(item, &format!("{at}[{i}]")).map(str::to_owned))
        .collect()
}

fn unique_strings(value: &Value, at: &str) -> Check<Vec<String>> {
    let values = strings(value, at)?;
    if values.iter().collect::<BTreeSet<_>>().len() != values.len() {
        return Err(format!("{at}: duplicate string value"));
    }
    Ok(values)
}

fn timestamp(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'Z'
        && [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18]
            .iter()
            .all(|i| b[*i].is_ascii_digit())
}

fn scalar_number_or_null(value: &Value, at: &str) -> Check<Option<f64>> {
    if value.is_null() {
        return Ok(None);
    }
    let n = value
        .as_f64()
        .ok_or_else(|| format!("{at}: expected numeric scalar or null"))?;
    if !n.is_finite() {
        return Err(format!("{at}: non-finite number"));
    }
    Ok(Some(n))
}

fn candidate_projection(case: &Value) -> Value {
    let as_of = case.get("as_of").and_then(Value::as_str).unwrap_or("");
    let mut projected_sites: Vec<Value> = case
        .get("sites")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|site| {
            let mut evidence: Vec<Value> = site
                .get("evidence")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|item| {
                    item.get("prospective_admissible").and_then(Value::as_bool) == Some(true)
                        && item
                            .get("observed_at")
                            .and_then(Value::as_str)
                            .is_some_and(|t| t <= as_of)
                })
                .cloned()
                .collect();
            evidence.sort_by(|a, b| {
                a.get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(b.get("name").and_then(Value::as_str).unwrap_or(""))
            });
            json!({
                "site_id": site.get("site_id"),
                "evidence": evidence
            })
        })
        .collect();
    projected_sites.sort_by(|a, b| {
        a.get("site_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("site_id").and_then(Value::as_str).unwrap_or(""))
    });
    json!({
        "case_id": case.get("case_id"),
        "as_of": case.get("as_of"),
        "protocol_id": case.get("protocol_id"),
        "required_enrollment": case.get("required_enrollment"),
        "sites": projected_sites
    })
}

fn numeric_evidence(site: &Value, name: &str, verified_direct_only: bool) -> Option<f64> {
    let records = site.get("evidence").and_then(Value::as_array)?;
    let item = records
        .iter()
        .find(|item| item.get("name").and_then(Value::as_str) == Some(name))?;
    if item.get("prospective_admissible").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    if verified_direct_only
        && (item.get("status").and_then(Value::as_str) != Some("verified")
            || item.get("source_kind").and_then(Value::as_str) != Some("direct_measurement"))
    {
        return None;
    }
    let value = item.get("value")?.as_f64()?;
    value.is_finite().then_some(value)
}

fn b1_self_report_target(site: &Value) -> Option<f64> {
    let records = site.get("evidence").and_then(Value::as_array)?;
    let item = records.iter().find(|item| {
        item.get("name").and_then(Value::as_str) == Some("site_projected_enrollment")
    })?;
    if item.get("status").and_then(Value::as_str) != Some("reported")
        || item.get("source_kind").and_then(Value::as_str) != Some("site_self_report")
        || item.get("prospective_admissible").and_then(Value::as_bool) != Some(true)
    {
        return None;
    }
    let value = item.get("value")?.as_f64()?;
    value.is_finite().then_some(value)
}

fn b3_readiness_signal(site: &Value) -> (Option<f64>, Vec<String>, String) {
    let staff = numeric_evidence(site, "current_staff_fte", true)
        .or_else(|| numeric_evidence(site, "staff_fte", true));
    let workflow = numeric_evidence(site, "workflow_coordination", true);
    let followup = numeric_evidence(site, "followup_reliability", true);
    let competing_load = numeric_evidence(site, "competing_trial_load", true);

    if let (Some(staff), Some(workflow)) = (staff, workflow) {
        let mut dimensions = vec![
            "staff_capacity".to_owned(),
            "workflow_coordination".to_owned(),
        ];
        let mut score = staff * workflow;
        if let Some(followup) = followup {
            score *= followup;
            dimensions.push("followup_reliability".to_owned());
        }
        if let Some(load) = competing_load {
            if (0.0..=1.0).contains(&load) {
                score *= 1.0 - load;
                dimensions.push("competing_trial_load_penalty".to_owned());
            }
        }
        return (
            Some(score),
            dimensions,
            "heuristic_readiness_signal_not_enrollment_forecast".to_owned(),
        );
    }

    if staff.is_none() {
        if let (Some(workflow), Some(followup)) = (workflow, followup) {
            return (
                Some(workflow * followup),
                vec![
                    "workflow_coordination".to_owned(),
                    "followup_reliability".to_owned(),
                ],
                "process_readiness_signal_access_dimensions_remain_separate".to_owned(),
            );
        }
    }

    let reason = if staff.is_none() {
        "abstain_missing_current_verified_staff_capacity".to_owned()
    } else if workflow.is_none() {
        "abstain_missing_verified_workflow_coordination".to_owned()
    } else {
        "abstain_insufficient_verified_capability_dimensions".to_owned()
    };
    (None, Vec::new(), reason)
}

fn top_site(values: &BTreeMap<String, f64>) -> Option<String> {
    values
        .iter()
        .max_by(|(id_a, score_a), (id_b, score_b)| {
            score_a.total_cmp(score_b).then_with(|| id_b.cmp(id_a))
        })
        .map(|(id, _)| id.clone())
}

fn access_evidence(site: &Value, name: &str) -> Value {
    site.get("evidence")
        .and_then(Value::as_array)
        .and_then(|records| {
            records
                .iter()
                .find(|item| item.get("name").and_then(Value::as_str) == Some(name))
        })
        .map(|item| {
            json!({
                "value": item.get("value"),
                "unit": item.get("unit"),
                "status": item.get("status"),
                "source_kind": item.get("source_kind"),
                "observed_at": item.get("observed_at")
            })
        })
        .unwrap_or(Value::Null)
}

fn evaluate_reference_methods(case: &Value, truth: &Value) -> Check<Value> {
    // Both candidate methods receive this projection only. Oracle outcomes are read
    // after predictions are formed and are used exclusively for evaluator metrics.
    let input = candidate_projection(case);
    let sites = array(
        field(&input, "sites", "candidate_projection")?,
        "candidate_projection.sites",
    )?;
    let mut b1_predictions = BTreeMap::<String, f64>::new();
    let mut b3_scores = BTreeMap::<String, f64>::new();
    let mut b1_rows = Vec::new();
    let mut b3_rows = Vec::new();
    let mut access_rows = Vec::new();

    for site in sites {
        let site_id = text_field(site, "site_id", "candidate_site")?.to_owned();
        if let Some(prediction) = b1_self_report_target(site) {
            b1_predictions.insert(site_id.clone(), prediction);
            b1_rows.push(json!({
                "site_id": site_id,
                "projected_enrollment": prediction,
                "signal_status": "self_reported_not_independently_verified"
            }));
        } else {
            b1_rows.push(json!({
                "site_id": site_id,
                "projected_enrollment": null,
                "signal_status": "abstain_missing_admissible_self_report"
            }));
        }

        let (score, dimensions, disposition) = b3_readiness_signal(site);
        if let Some(score) = score {
            b3_scores.insert(site_id.clone(), score);
            b3_rows.push(json!({
                "site_id": site_id,
                "readiness_signal": score,
                "dimensions_used": dimensions,
                "disposition": disposition
            }));
        } else {
            b3_rows.push(json!({
                "site_id": site_id,
                "readiness_signal": null,
                "dimensions_used": dimensions,
                "disposition": disposition
            }));
        }

        access_rows.push(json!({
            "site_id": site_id,
            "access_barrier_rate": access_evidence(site, "access_barrier_rate"),
            "accessible_contact_channel_coverage": access_evidence(site, "accessible_contact_channel_coverage")
        }));
    }

    let outcomes = array(
        field(truth, "actual_outcomes_by_site", "oracle_truth")?,
        "oracle_truth.actual_outcomes_by_site",
    )?;
    let mut actual_enrollment = BTreeMap::<String, f64>::new();
    for outcome in outcomes {
        let id = text_field(outcome, "site_id", "oracle_outcome")?.to_owned();
        let enrolled = field(outcome, "enrolled", "oracle_outcome")?
            .as_f64()
            .ok_or_else(|| "oracle outcome enrolled value is not numeric".to_owned())?;
        actual_enrollment.insert(id, enrolled);
    }
    let actual_top_site = top_site(&actual_enrollment);
    let b1_top_site = top_site(&b1_predictions);
    let b3_top_site = top_site(&b3_scores);

    let absolute_errors: Vec<f64> = b1_predictions
        .iter()
        .filter_map(|(id, prediction)| {
            actual_enrollment
                .get(id)
                .map(|actual| (prediction - actual).abs())
        })
        .collect();
    let b1_mae = if absolute_errors.is_empty() {
        Value::Null
    } else {
        json!(absolute_errors.iter().sum::<f64>() / absolute_errors.len() as f64)
    };
    let b1_top_matches_actual = b1_top_site
        .as_ref()
        .zip(actual_top_site.as_ref())
        .map(|(predicted, actual)| predicted == actual);
    let b3_top_matches_actual = b3_top_site
        .as_ref()
        .zip(actual_top_site.as_ref())
        .map(|(predicted, actual)| predicted == actual);
    let b3_comparable_pair = b3_scores.len() == sites.len();
    let b3_directional_concordance = if b3_comparable_pair && sites.len() == 2 {
        let ids: Vec<String> = b3_scores.keys().cloned().collect();
        let (a, b) = (&ids[0], &ids[1]);
        let score_diff = b3_scores[a] - b3_scores[b];
        let outcome_diff = actual_enrollment[a] - actual_enrollment[b];
        Some(if score_diff == 0.0 || outcome_diff == 0.0 {
            0.5
        } else if score_diff.signum() == outcome_diff.signum() {
            1.0
        } else {
            0.0
        })
    } else {
        None
    };

    Ok(json!({
        "candidate_input": "prospectively_admissible_evidence_at_or_before_as_of_only",
        "B1_self_report_only": {
            "site_signals": b1_rows,
            "enrollment_mae_against_synthetic_truth": b1_mae,
            "forecast_count": b1_predictions.len(),
            "top_reported_target_site": b1_top_site,
            "top_target_matches_top_synthetic_enrollment": b1_top_matches_actual
        },
        "B3_capability_aware_heuristic": {
            "site_signals": b3_rows,
            "rankable_site_count": b3_scores.len(),
            "abstention_count": sites.len().saturating_sub(b3_scores.len()),
            "top_readiness_signal_site": b3_top_site,
            "top_readiness_site_matches_top_synthetic_enrollment": b3_top_matches_actual,
            "pairwise_directional_concordance_with_synthetic_enrollment": b3_directional_concordance,
            "claim_boundary": "uncalibrated readiness heuristic; not an enrollment forecast"
        },
        "separate_access_dimensions": access_rows,
        "evaluation_metadata": {
            "site_count": sites.len(),
            "synthetic_actual_top_enrollment_site": actual_top_site,
            "site_targets_and_readiness_scores_are_not_probabilities": true
        }
    }))
}

fn validate_site_truth_ids(ids: &[String], sites: &BTreeSet<String>, at: &str) -> Check<()> {
    for id in ids {
        if !sites.contains(id) {
            return Err(format!("{at}: site ID {id} is not in the case"));
        }
    }
    Ok(())
}

fn validate_case(case: &Value, index: usize) -> Check<Value> {
    let at = format!("cases[{index}]");
    let case_keys = [
        "case_id",
        "as_of",
        "protocol_id",
        "required_enrollment",
        "sites",
        "oracle_truth",
        "expected_invariants",
    ];
    require_keys(case, &case_keys, &case_keys, &at)?;
    let case_id = text_field(case, "case_id", &at)?;
    let as_of = text_field(case, "as_of", &at)?;
    if !timestamp(as_of) {
        return Err(format!(
            "{at}.as_of: expected UTC RFC3339 seconds timestamp"
        ));
    }
    if text_field(case, "protocol_id", &at)?.is_empty() {
        return Err(format!("{at}.protocol_id: empty protocol identity"));
    }
    let required_enrollment = field(case, "required_enrollment", &at)?
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or_else(|| format!("{at}.required_enrollment: expected positive integer"))?;

    let sites = array(field(case, "sites", &at)?, &format!("{at}.sites"))?;
    if sites.is_empty() {
        return Err(format!("{at}.sites: empty site set"));
    }
    let mut site_ids = BTreeSet::new();
    for (site_index, site) in sites.iter().enumerate() {
        let site_at = format!("{at}.sites[{site_index}]");
        require_keys(
            site,
            &["site_id", "evidence"],
            &["site_id", "evidence"],
            &site_at,
        )?;
        let site_id = text_field(site, "site_id", &site_at)?.to_owned();
        if site_id.is_empty() || !site_ids.insert(site_id.clone()) {
            return Err(format!("{site_at}: empty or duplicate site ID {site_id}"));
        }
        let records = array(
            field(site, "evidence", &site_at)?,
            &format!("{site_at}.evidence"),
        )?;
        let mut feature_names = BTreeSet::new();
        for (feature_index, evidence) in records.iter().enumerate() {
            let evidence_at = format!("{site_at}.evidence[{feature_index}]");
            let evidence_keys = [
                "name",
                "value",
                "unit",
                "status",
                "source_kind",
                "observed_at",
                "prospective_admissible",
            ];
            require_keys(evidence, &evidence_keys, &evidence_keys, &evidence_at)?;
            let name = text_field(evidence, "name", &evidence_at)?;
            if name.is_empty() || !feature_names.insert(name.to_owned()) {
                return Err(format!(
                    "{evidence_at}: empty or duplicate feature name {name}"
                ));
            }
            if text_field(evidence, "unit", &evidence_at)?.is_empty() {
                return Err(format!("{evidence_at}.unit: empty unit"));
            }
            let status = text_field(evidence, "status", &evidence_at)?;
            if !STATUSES.contains(&status) {
                return Err(format!("{evidence_at}: invalid evidence status {status}"));
            }
            let source_kind = text_field(evidence, "source_kind", &evidence_at)?;
            if !SOURCE_KINDS.contains(&source_kind) {
                return Err(format!("{evidence_at}: invalid source kind {source_kind}"));
            }
            let observed_at = text_field(evidence, "observed_at", &evidence_at)?;
            if !timestamp(observed_at) {
                return Err(format!("{evidence_at}.observed_at: invalid UTC timestamp"));
            }
            let admissible = field(evidence, "prospective_admissible", &evidence_at)?
                .as_bool()
                .ok_or_else(|| format!("{evidence_at}.prospective_admissible: expected boolean"))?;
            let value = scalar_number_or_null(
                field(evidence, "value", &evidence_at)?,
                &format!("{evidence_at}.value"),
            )?;
            if status == "unknown" && value.is_some() {
                return Err(format!(
                    "{evidence_at}: unknown evidence must not carry a numeric value"
                ));
            }
            if admissible && observed_at > as_of {
                return Err(format!(
                    "{evidence_at}: future evidence marked prospectively admissible"
                ));
            }
            if source_kind == "post_selection_outcome" && (admissible || observed_at <= as_of) {
                return Err(format!(
                    "{evidence_at}: post-selection outcome must be future-dated and inadmissible"
                ));
            }
            if status == "verified" && value.is_none() {
                return Err(format!(
                    "{evidence_at}: verified evidence cannot have a null value"
                ));
            }
        }
    }

    let truth = field(case, "oracle_truth", &at)?;
    let truth_keys = [
        "actual_outcomes_by_site",
        "primary_bottleneck_by_site",
        "support_targets",
    ];
    require_keys(
        truth,
        &truth_keys,
        &truth_keys,
        &format!("{at}.oracle_truth"),
    )?;
    let outcomes = array(
        field(truth, "actual_outcomes_by_site", &at)?,
        &format!("{at}.oracle_truth.actual_outcomes_by_site"),
    )?;
    if outcomes.len() != sites.len() {
        return Err(format!(
            "{at}.oracle_truth: outcomes must account for every site"
        ));
    }
    let mut outcome_ids = BTreeSet::new();
    for (outcome_index, outcome) in outcomes.iter().enumerate() {
        let out_at = format!("{at}.oracle_truth.actual_outcomes_by_site[{outcome_index}]");
        require_keys(
            outcome,
            &[
                "site_id",
                "enrolled",
                "retained",
                "access_coverage_score",
                "protocol_deviations",
            ],
            &[
                "site_id",
                "enrolled",
                "retained",
                "access_coverage_score",
                "protocol_deviations",
            ],
            &out_at,
        )?;
        let site_id = text_field(outcome, "site_id", &out_at)?.to_owned();
        if !site_ids.contains(&site_id) || !outcome_ids.insert(site_id.clone()) {
            return Err(format!(
                "{out_at}: unknown or duplicate outcome site ID {site_id}"
            ));
        }
        let enrolled = field(outcome, "enrolled", &out_at)?
            .as_u64()
            .ok_or_else(|| format!("{out_at}.enrolled: expected non-negative integer"))?;
        let retained = field(outcome, "retained", &out_at)?
            .as_u64()
            .ok_or_else(|| format!("{out_at}.retained: expected non-negative integer"))?;
        field(outcome, "protocol_deviations", &out_at)?
            .as_u64()
            .ok_or_else(|| {
                format!("{out_at}.protocol_deviations: expected non-negative integer")
            })?;
        let access = field(outcome, "access_coverage_score", &out_at)?
            .as_f64()
            .ok_or_else(|| format!("{out_at}.access_coverage_score: expected number"))?;
        if retained > enrolled || !(0.0..=1.0).contains(&access) {
            return Err(format!(
                "{out_at}: invalid retained/enrolled relation or access score"
            ));
        }
    }
    let bottlenecks = array(
        field(truth, "primary_bottleneck_by_site", &at)?,
        &format!("{at}.oracle_truth.primary_bottleneck_by_site"),
    )?;
    for (i, bottleneck) in bottlenecks.iter().enumerate() {
        let p = format!("{at}.oracle_truth.primary_bottleneck_by_site[{i}]");
        require_keys(bottleneck, &["site_id", "name"], &["site_id", "name"], &p)?;
        if !site_ids.contains(text_field(bottleneck, "site_id", &p)?) {
            return Err(format!("{p}: unknown site ID"));
        }
        if text_field(bottleneck, "name", &p)?.is_empty() {
            return Err(format!("{p}: empty bottleneck name"));
        }
    }
    let targets = array(
        field(truth, "support_targets", &at)?,
        &format!("{at}.oracle_truth.support_targets"),
    )?;
    for (i, target) in targets.iter().enumerate() {
        let p = format!("{at}.oracle_truth.support_targets[{i}]");
        require_keys(
            target,
            &["site_id", "intervention"],
            &["site_id", "intervention"],
            &p,
        )?;
        if !site_ids.contains(text_field(target, "site_id", &p)?) {
            return Err(format!("{p}: unknown site ID"));
        }
        if text_field(target, "intervention", &p)?.is_empty() {
            return Err(format!("{p}: empty intervention"));
        }
    }
    let invariants = unique_strings(
        field(case, "expected_invariants", &at)?,
        &format!("{at}.expected_invariants"),
    )?;
    if invariants.is_empty() {
        return Err(format!("{at}.expected_invariants: empty invariant set"));
    }

    if case_id == "OUTCOME-LEAKAGE-006" {
        let leaked_feature = sites
            .iter()
            .flat_map(|site| {
                site.get("evidence")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
            })
            .any(|e| {
                e.get("source_kind").and_then(Value::as_str) == Some("post_selection_outcome")
            });
        let projection = candidate_projection(case);
        let projection_contains_future_outcome =
            projection.to_string().contains("post_selection_outcome");
        if !leaked_feature || projection_contains_future_outcome {
            return Err(format!("{at}: future outcome leakage sentinel failed"));
        }
    }

    Ok(json!({
        "case_id": case_id,
        "status": "pass",
        "site_count": sites.len(),
        "required_enrollment": required_enrollment,
        "candidate_projection_sha256": format!("{:x}", Sha256::digest(serde_json::to_vec(&candidate_projection(case)).unwrap_or_default())),
        "reference_comparison": evaluate_reference_methods(case, truth)?
    }))
}

fn validate_corpus(corpus: &Value) -> Check<Vec<Value>> {
    let keys = [
        "schema_version",
        "benchmark_id",
        "classification",
        "created_at",
        "source_boundaries",
        "admissible_evidence_statuses",
        "admissible_source_kinds",
        "cases",
    ];
    require_keys(corpus, &keys, &keys, "corpus")?;
    if text_field(corpus, "schema_version", "corpus")? != "0.1.0"
        || text_field(corpus, "benchmark_id", "corpus")? != "TRIAL-SITE-OPS-001"
        || text_field(corpus, "classification", "corpus")? != "synthetic_only"
    {
        return Err("corpus: schema/version/identity/classification mismatch".to_owned());
    }
    let status_values: BTreeSet<String> = strings(
        field(corpus, "admissible_evidence_statuses", "corpus")?,
        "corpus.admissible_evidence_statuses",
    )?
    .into_iter()
    .collect();
    let expected_statuses: BTreeSet<String> = STATUSES.iter().map(|s| (*s).to_owned()).collect();
    if status_values != expected_statuses {
        return Err("corpus: evidence status vocabulary mismatch".to_owned());
    }
    let source_values: BTreeSet<String> = strings(
        field(corpus, "admissible_source_kinds", "corpus")?,
        "corpus.admissible_source_kinds",
    )?
    .into_iter()
    .collect();
    let expected_sources: BTreeSet<String> = SOURCE_KINDS.iter().map(|s| (*s).to_owned()).collect();
    if source_values != expected_sources {
        return Err("corpus: source kind vocabulary mismatch".to_owned());
    }
    let boundaries = field(corpus, "source_boundaries", "corpus")?;
    require_keys(
        boundaries,
        &[
            "candidate_input_projection",
            "oracle_truth_is_hidden",
            "rationale",
        ],
        &[
            "candidate_input_projection",
            "oracle_truth_is_hidden",
            "rationale",
        ],
        "corpus.source_boundaries",
    )?;
    if boundaries
        .get("oracle_truth_is_hidden")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Err("corpus.source_boundaries: oracle truth must be hidden".to_owned());
    }
    let created_at = text_field(corpus, "created_at", "corpus")?;
    if !timestamp(created_at) {
        return Err("corpus.created_at: invalid UTC timestamp".to_owned());
    }
    let cases = array(field(corpus, "cases", "corpus")?, "corpus.cases")?;
    let ids: Vec<String> = cases
        .iter()
        .enumerate()
        .map(|(i, c)| text_field(c, "case_id", &format!("cases[{i}]")).map(str::to_owned))
        .collect::<Check<Vec<_>>>()?;
    if ids.len() != CASE_IDS.len()
        || ids
            .iter()
            .zip(CASE_IDS.iter())
            .any(|(a, b)| a.as_str() != *b)
    {
        return Err(format!(
            "corpus: case inventory/order must equal {CASE_IDS:?}"
        ));
    }
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err("corpus: duplicate case ID".to_owned());
    }
    cases
        .iter()
        .enumerate()
        .map(|(i, c)| validate_case(c, i))
        .collect()
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err(
            "usage: trial-site-ops-bench <fixture.json> --source-commit <40-hex-sha>".to_owned(),
        );
    }
    let source_commit = &args[3];
    if source_commit.len() != 40 || !source_commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character hexadecimal SHA".to_owned());
    }
    let bytes = fs::read(&args[1]).map_err(|e| format!("read {}: {e}", args[1]))?;
    let source = std::str::from_utf8(&bytes).map_err(|e| format!("fixture is not UTF-8: {e}"))?;
    let corpus: Value =
        serde_json::from_str(source).map_err(|e| format!("parse fixture JSON: {e}"))?;
    let scenarios = validate_corpus(&corpus)?;
    let receipt = json!({
        "result":"pass",
        "validator_version":VERSION,
        "benchmark_id":"TRIAL-SITE-OPS-001",
        "scenario_count":scenarios.len(),
        "corpus_sha256":format!("{:x}",Sha256::digest(&bytes)),
        "source_commit":source_commit,
        "scenarios":scenarios
    });
    println!(
        "{}",
        serde_json::to_string(&receipt).map_err(|e| format!("serialize receipt: {e}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("trial-site-ops-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE: &str = include_str!("../../../benchmarks/fixtures/TRIAL_SITE_OPS_V0_1.json");
    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture must parse")
    }

    #[test]
    fn frozen_corpus_passes_all_six_cases() {
        let result = validate_corpus(&fixture()).expect("frozen synthetic corpus should validate");
        assert_eq!(result.len(), 6);
        assert!(
            result
                .iter()
                .all(|r| r.get("status").and_then(Value::as_str) == Some("pass"))
        );
    }

    #[test]
    fn compares_self_report_targets_with_capability_signals_on_capability_gap_case() {
        let corpus = fixture();
        let scenarios = validate_corpus(&corpus).expect("fixture validates");
        let case = scenarios
            .iter()
            .find(|item| item.get("case_id").and_then(Value::as_str) == Some("CAPABILITY-GAP-001"))
            .expect("capability gap scenario exists");
        let comparison = &case["reference_comparison"];
        assert_eq!(
            comparison["B1_self_report_only"]["top_reported_target_site"].as_str(),
            Some("SITE-NORTH")
        );
        assert_eq!(
            comparison["B1_self_report_only"]["top_target_matches_top_synthetic_enrollment"]
                .as_bool(),
            Some(false)
        );
        assert_eq!(
            comparison["B3_capability_aware_heuristic"]["top_readiness_signal_site"].as_str(),
            Some("SITE-SOUTH")
        );
        assert_eq!(
            comparison["B3_capability_aware_heuristic"]["top_readiness_site_matches_top_synthetic_enrollment"].as_bool(),
            Some(true)
        );
        assert_eq!(
            comparison["B3_capability_aware_heuristic"]["pairwise_directional_concordance_with_synthetic_enrollment"].as_f64(),
            Some(1.0)
        );
    }

    #[test]
    fn stale_and_incomplete_case_abstains_instead_of_zero_filling() {
        let corpus = fixture();
        let scenarios = validate_corpus(&corpus).expect("fixture validates");
        let case = scenarios
            .iter()
            .find(|item| {
                item.get("case_id").and_then(Value::as_str) == Some("STALE-INCOMPLETE-005")
            })
            .expect("stale case exists");
        assert_eq!(
            case["reference_comparison"]["B3_capability_aware_heuristic"]["rankable_site_count"]
                .as_u64(),
            Some(0)
        );
        assert_eq!(
            case["reference_comparison"]["B3_capability_aware_heuristic"]["abstention_count"]
                .as_u64(),
            Some(2)
        );
    }

    #[test]
    fn candidate_projection_and_reference_comparison_ignore_input_order() {
        let original = fixture();
        let original_results = validate_corpus(&original).expect("original fixture validates");
        let mut reordered = original.clone();
        for case in reordered["cases"].as_array_mut().expect("case array") {
            case["sites"].as_array_mut().expect("site array").reverse();
            for site in case["sites"].as_array_mut().expect("site array") {
                site["evidence"]
                    .as_array_mut()
                    .expect("evidence array")
                    .reverse();
            }
        }
        let reordered_results = validate_corpus(&reordered).expect("reordered fixture validates");
        assert_eq!(original_results, reordered_results);
    }

    #[test]
    fn access_dimensions_are_reported_separately_from_readiness_scores() {
        let corpus = fixture();
        let scenarios = validate_corpus(&corpus).expect("fixture validates");
        let case = scenarios
            .iter()
            .find(|item| {
                item.get("case_id").and_then(Value::as_str) == Some("REPRESENTATION-ACCESS-004")
            })
            .expect("access scenario exists");
        let comparison = &case["reference_comparison"];
        assert!(comparison.get("separate_access_dimensions").is_some());
        assert!(comparison.get("site_quality_score").is_none());
        let access_rows = comparison["separate_access_dimensions"]
            .as_array()
            .expect("access rows");
        assert_eq!(access_rows.len(), 2);
        assert!(access_rows.iter().any(|row| {
            row["site_id"].as_str() == Some("SITE-COMMUNITY")
                && row["accessible_contact_channel_coverage"]["value"].as_f64() == Some(0.88)
        }));
    }

    #[test]
    fn candidate_projection_excludes_truth_and_expected_invariants() {
        let projected = candidate_projection(&fixture()["cases"][0]);
        assert!(projected.get("oracle_truth").is_none());
        assert!(projected.get("expected_invariants").is_none());
    }

    #[test]
    fn candidate_projection_excludes_post_selection_outcome() {
        let projection = candidate_projection(&fixture()["cases"][5]);
        assert!(!projection.to_string().contains("post_selection_outcome"));
    }

    #[test]
    fn rejects_future_evidence_marked_admissible() {
        let mut corpus = fixture();
        corpus["cases"][0]["as_of"] = json!("2026-02-01T00:00:00Z");
        let error = validate_corpus(&corpus).expect_err("future evidence must not be admitted");
        assert!(
            error.contains("future evidence marked prospectively admissible"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_unknown_evidence_with_numeric_value() {
        let mut corpus = fixture();
        corpus["cases"][0]["sites"][0]["evidence"][0]["status"] = json!("unknown");
        let error = validate_corpus(&corpus).expect_err("unknown values cannot be numeric");
        assert!(
            error.contains("unknown evidence must not carry a numeric value"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_duplicate_site_ids() {
        let mut corpus = fixture();
        corpus["cases"][0]["sites"][1]["site_id"] = json!("SITE-NORTH");
        let error = validate_corpus(&corpus).expect_err("duplicate sites must fail");
        assert!(
            error.contains("duplicate site ID"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_post_selection_outcome_marked_admissible() {
        let mut corpus = fixture();
        corpus["cases"][5]["sites"][0]["evidence"][3]["prospective_admissible"] = json!(true);
        let error = validate_corpus(&corpus).expect_err("post-selection outcomes must be excluded");
        assert!(
            error.contains("future evidence marked prospectively admissible")
                || error.contains("post-selection outcome must be future-dated and inadmissible"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_changed_case_inventory() {
        let mut corpus = fixture();
        corpus["cases"][0]["case_id"] = json!("OUTCOME-LEAKAGE-006");
        let error = validate_corpus(&corpus).expect_err("case inventory is frozen");
        assert!(
            error.contains("case inventory"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_unknown_source_kind() {
        let mut corpus = fixture();
        corpus["cases"][0]["sites"][0]["evidence"][0]["source_kind"] = json!("oracle_truth");
        let error = validate_corpus(&corpus).expect_err("unrecognized source kind must fail");
        assert!(
            error.contains("invalid source kind"),
            "unexpected error: {error}"
        );
    }
}

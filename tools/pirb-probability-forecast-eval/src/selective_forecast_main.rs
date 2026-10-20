// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Audits reason-coded selective forecast dispositions. An abstention is not a
// no-event prediction, and unknown outcomes are never imputed.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-selective-forecast-v0.1.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-selective-forecast-synthetic-v0.1.json");

const DISPOSITIONS: &[&str] = &[
    "scored",
    "abstain_insufficient_evidence",
    "abstain_stale_inputs",
    "abstain_out_of_support",
    "abstain_target_mismatch",
    "not_scored_unknown_outcome",
];
const NON_SCORED_DISPOSITIONS: &[&str] = &[
    "abstain_insufficient_evidence",
    "abstain_stale_inputs",
    "abstain_out_of_support",
    "abstain_target_mismatch",
    "not_scored_unknown_outcome",
];

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn option_value(args: &[String], name: &str) -> Result<Option<String>, String> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("argument {name} requires a value"))
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn semantic_errors(registration: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(records) = registration.get("records").and_then(Value::as_array) else {
        return vec!["records must be an array after schema validation".to_owned()];
    };
    let Some(coverage_levels) = registration["preregistration"]["coverage_levels"].as_array()
    else {
        return vec!["coverage_levels must be an array after schema validation".to_owned()];
    };

    let levels = coverage_levels
        .iter()
        .filter_map(Value::as_f64)
        .collect::<Vec<_>>();
    if levels.last().is_none_or(|level| (*level - 1.0).abs() > f64::EPSILON) {
        errors.push("coverage_levels must end at 1.0".to_owned());
    }
    if levels.windows(2).any(|pair| pair[0] >= pair[1]) {
        errors.push("coverage_levels must be strictly increasing".to_owned());
    }

    let mut record_ids = BTreeSet::new();
    for record in records {
        let id = string_at(record, "record_id").unwrap_or_default();
        if !record_ids.insert(id.to_owned()) {
            errors.push(format!("record ID is duplicated: {id}"));
        }

        let disposition = string_at(record, "disposition").unwrap_or_default();
        let outcome = string_at(record, "outcome").unwrap_or_default();
        let evidence = string_at(record, "evidence_status").unwrap_or_default();
        let freshness = string_at(record, "input_freshness").unwrap_or_default();
        let support = string_at(record, "support_status").unwrap_or_default();
        let target_matched = record["target_matched"].as_bool() == Some(true);
        let probability_present = !record["forecast_probability"].is_null();
        let selection_score_present = !record["selection_score"].is_null();

        if disposition == "scored" {
            if outcome == "unknown" {
                errors.push(format!(
                    "record {id} cannot be scored with an unknown outcome"
                ));
            }
            if !probability_present || !selection_score_present {
                errors.push(format!(
                    "scored record {id} requires both forecast_probability and selection_score"
                ));
            }
            if evidence != "sufficient" {
                errors.push(format!(
                    "scored record {id} requires sufficient evidence"
                ));
            }
            if freshness != "fresh" {
                errors.push(format!("scored record {id} requires fresh inputs"));
            }
            if support != "in_support" {
                errors.push(format!("scored record {id} must be in the supported input domain"));
            }
            if !target_matched {
                errors.push(format!("scored record {id} must match the registered target"));
            }
        } else {
            if probability_present || selection_score_present {
                errors.push(format!(
                    "non-scored record {id} must not publish a forecast probability or selection score"
                ));
            }
            match disposition {
                "abstain_insufficient_evidence" if evidence != "insufficient" => errors.push(
                    format!(
                        "record {id} uses abstain_insufficient_evidence without insufficient evidence"
                    ),
                ),
                "abstain_stale_inputs" if freshness != "stale" => {
                    errors.push(format!("record {id} uses abstain_stale_inputs without stale inputs"))
                }
                "abstain_out_of_support" if support != "out_of_support" => errors.push(format!(
                    "record {id} uses abstain_out_of_support while input is marked in support"
                )),
                "abstain_target_mismatch" if target_matched => errors.push(format!(
                    "record {id} uses abstain_target_mismatch while target_matched is true"
                )),
                "not_scored_unknown_outcome" if outcome != "unknown" => errors.push(format!(
                    "record {id} uses not_scored_unknown_outcome with a known outcome"
                )),
                _ => {}
            }
            if disposition == "not_scored_unknown_outcome"
                && (evidence != "sufficient"
                    || freshness != "fresh"
                    || support != "in_support"
                    || !target_matched)
            {
                errors.push(format!(
                    "record {id} must use the primary evidence/target abstention reason before not_scored_unknown_outcome"
                ));
            }
        }
    }

    if records.is_empty() {
        errors.push("selective forecast registration has no records".to_owned());
    }
    errors.sort();
    errors
}

fn scored_outcome(record: &Value) -> f64 {
    if string_at(record, "outcome") == Some("event") {
        1.0
    } else {
        0.0
    }
}

fn brier_for(records: &[&Value]) -> Option<f64> {
    if records.is_empty() {
        return None;
    }
    Some(
        records
            .iter()
            .map(|record| {
                let probability =
                    number_at(record, "forecast_probability").expect("scored records have p");
                let difference = probability - scored_outcome(record);
                difference * difference
            })
            .sum::<f64>()
            / records.len() as f64,
    )
}

fn log_loss_for(records: &[&Value], epsilon: f64) -> Option<f64> {
    if records.is_empty() {
        return None;
    }
    Some(
        records
            .iter()
            .map(|record| {
                let probability =
                    number_at(record, "forecast_probability").expect("scored records have p");
                let clipped = probability.clamp(epsilon, 1.0 - epsilon);
                if scored_outcome(record) == 1.0 {
                    -clipped.ln()
                } else {
                    -(1.0 - clipped).ln()
                }
            })
            .sum::<f64>()
            / records.len() as f64,
    )
}

fn event_prevalence(records: &[&Value]) -> Option<f64> {
    if records.is_empty() {
        return None;
    }
    Some(
        records
            .iter()
            .filter(|record| string_at(record, "outcome") == Some("event"))
            .count() as f64
            / records.len() as f64,
    )
}

fn score_value(value: Option<f64>) -> Value {
    value.map_or(Value::Null, |number| json!(number))
}

fn risk_coverage_curve(
    scored: &[&Value],
    coverage_levels: &[f64],
    epsilon: f64,
) -> Vec<Value> {
    if scored.is_empty() {
        return Vec::new();
    }
    let mut ranked = scored.to_vec();
    ranked.sort_by(|left, right| {
        let left_score = number_at(left, "selection_score").expect("scored rows have rank score");
        let right_score =
            number_at(right, "selection_score").expect("scored rows have rank score");
        right_score
            .partial_cmp(&left_score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                string_at(left, "record_id")
                    .cmp(&string_at(right, "record_id"))
            })
    });

    coverage_levels
        .iter()
        .map(|requested| {
            let retained_count = ((*requested * ranked.len() as f64).ceil() as usize)
                .max(1)
                .min(ranked.len());
            let retained = &ranked[..retained_count];
            let threshold = number_at(
                retained
                    .last()
                    .expect("positive preregistered coverage retains a record"),
                "selection_score",
            )
            .expect("scored rows have a selection score");
            json!({
                "requested_coverage": requested,
                "retained_count": retained_count,
                "scored_subset_count": ranked.len(),
                "realized_scored_subset_coverage": retained_count as f64 / ranked.len() as f64,
                "selection_score_at_boundary": threshold,
                "brier_score": score_value(brier_for(retained)),
                "log_loss": score_value(log_loss_for(retained, epsilon)),
                "event_prevalence": score_value(event_prevalence(retained)),
                "tie_break_policy": "record_id_ascending"
            })
        })
        .collect()
}

fn disposition_summary(records: &[&Value], disposition: &str) -> Value {
    let selected = records
        .iter()
        .copied()
        .filter(|record| string_at(record, "disposition") == Some(*disposition))
        .collect::<Vec<_>>();
    let event_count = selected
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("event"))
        .count();
    let no_event_count = selected
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("no_event"))
        .count();
    let unknown_outcome_count = selected
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("unknown"))
        .count();
    let known_outcome_count = event_count + no_event_count;
    let known_prevalence = if known_outcome_count == 0 {
        Value::Null
    } else {
        json!(event_count as f64 / known_outcome_count as f64)
    };
    let unknown_rate = if selected.is_empty() {
        Value::Null
    } else {
        json!(unknown_outcome_count as f64 / selected.len() as f64)
    };

    json!({
        "disposition": disposition,
        "record_count": selected.len(),
        "event_count": event_count,
        "no_event_count": no_event_count,
        "unknown_outcome_count": unknown_outcome_count,
        "known_outcome_count": known_outcome_count,
        "event_prevalence_among_known_outcomes": known_prevalence,
        "unknown_outcome_rate": unknown_rate
    })
}

fn group_summaries(records: &[&Value], minimum: usize, epsilon: f64) -> (Vec<Value>, usize) {
    let mut by_group: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for record in records {
        by_group
            .entry(string_at(record, "group_id").expect("schema requires group_id"))
            .or_default()
            .push(*record);
    }

    let mut results = Vec::new();
    let mut suppressed = 0usize;
    for (group_id, group_records) in by_group {
        if group_records.len() < minimum {
            suppressed += 1;
            continue;
        }
        let scored = group_records
            .iter()
            .copied()
            .filter(|record| string_at(record, "disposition") == Some("scored"))
            .collect::<Vec<_>>();
        let unknown_outcome_count = group_records
            .iter()
            .filter(|record| string_at(record, "outcome") == Some("unknown"))
            .count();
        results.push(json!({
            "group_id": group_id,
            "record_count": group_records.len(),
            "scored_count": scored.len(),
            "non_scored_count": group_records.len() - scored.len(),
            "actual_score_coverage": scored.len() as f64 / group_records.len() as f64,
            "unknown_outcome_count": unknown_outcome_count,
            "brier_score_on_scored": score_value(brier_for(&scored)),
            "log_loss_on_scored": score_value(log_loss_for(&scored, epsilon)),
            "event_prevalence_on_scored": score_value(event_prevalence(&scored))
        }));
    }
    (results, suppressed)
}

fn validate_and_audit(input: &[u8], source_commit: &str) -> Result<Value, Vec<String>> {
    if source_commit != "UNPINNED_LOCAL_RUN"
        && (source_commit.len() != 40
            || !source_commit
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()))
    {
        return Err(vec![
            "source commit must be a 40-character lowercase Git SHA or UNPINNED_LOCAL_RUN"
                .to_owned(),
        ]);
    }

    let registration: Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("selective forecast registration JSON is invalid: {error}")])?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES)
        .map_err(|error| vec![format!("embedded selective forecast schema is invalid: {error}")])?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!("Draft 2020-12 schema meta-validation failed: {error}")]
    })?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| vec![format!("cannot build selective forecast validator: {error}")])?;
    let mut errors = validator
        .iter_errors(&registration)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        errors.sort();
        return Err(errors);
    }
    let semantic = semantic_errors(&registration);
    if !semantic.is_empty() {
        return Err(semantic);
    }

    let records = registration["records"]
        .as_array()
        .expect("schema validation requires records")
        .iter()
        .collect::<Vec<_>>();
    let scored = records
        .iter()
        .copied()
        .filter(|record| string_at(record, "disposition") == Some("scored"))
        .collect::<Vec<_>>();
    let epsilon = number_at(
        &registration["preregistration"],
        "probability_clip_epsilon",
    )
    .expect("schema validation requires probability_clip_epsilon");
    let coverage_levels = registration["preregistration"]["coverage_levels"]
        .as_array()
        .expect("schema validation requires coverage_levels")
        .iter()
        .filter_map(Value::as_f64)
        .collect::<Vec<_>>();

    let mut disposition_counts = BTreeMap::new();
    for disposition in DISPOSITIONS {
        disposition_counts.insert(
            (*disposition).to_owned(),
            records
                .iter()
                .filter(|record| string_at(record, "disposition") == Some(disposition))
                .count(),
        );
    }
    let non_scored_summaries = NON_SCORED_DISPOSITIONS
        .iter()
        .map(|disposition| disposition_summary(&records, disposition))
        .collect::<Vec<_>>();
    let unknown_outcome_count = records
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("unknown"))
        .count();
    let total_count = records.len();
    let actual_coverage = scored.len() as f64 / total_count as f64;
    let (groups, suppressed_group_count) = group_summaries(
        &records,
        registration["privacy"]["group_min_count"]
            .as_u64()
            .expect("schema validation requires group_min_count") as usize,
        epsilon,
    );
    let risk_curve = risk_coverage_curve(&scored, &coverage_levels, epsilon);

    let registration_id =
        string_at(&registration, "registration_id").expect("schema validates registration_id");
    let input_sha256 = sha256_hex(input);
    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let forecast_archive_sha256 =
        string_at(&registration["provider"], "forecast_archive_sha256")
            .expect("schema validates archive digest");
    let outcome_snapshot_sha256 =
        string_at(&registration["provider"], "outcome_snapshot_sha256")
            .expect("schema validates outcome digest");
    let receipt_commitment_sha256 = sha256_hex(
        format!(
            "pirb-selective-forecast-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "selective_forecast_abstention_audit",
        "registration_id": registration_id,
        "evidence_scope": registration["evidence_scope"],
        "provider": registration["provider"]["provider_id"],
        "forecast_archive_sha256": forecast_archive_sha256,
        "outcome_snapshot_sha256": outcome_snapshot_sha256,
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "total_record_count": total_count,
        "scored_record_count": scored.len(),
        "actual_score_coverage": actual_coverage,
        "unknown_outcome_count": unknown_outcome_count,
        "disposition_counts": disposition_counts,
        "non_scored_disposition_summary": non_scored_summaries,
        "risk_coverage_curve": risk_curve,
        "group_metrics": groups,
        "suppressed_group_count": suppressed_group_count,
        "metric_semantics": {
            "risk_coverage_population": "rows that passed all evidence gates and have known outcomes",
            "risk_coverage_order": "selection_score_desc_then_record_id_asc",
            "actual_score_coverage_denominator": "all registered rows",
            "unknown_outcomes_imputed_to_no_event": false,
            "abstention_means_no_event": false,
            "threshold_selected_on_test": false,
            "score_policy": registration["preregistration"]["score_policy"]
        },
        "claims": {
            "contract_validated": true,
            "selective_score_diagnostics_computed": true,
            "distribution_shift_detector_quality_established": false,
            "fairness_established": false,
            "operational_utility_established": false,
            "causal_impact_established": false,
            "live_decision_authority": false
        },
        "receipt_commitment_sha256": receipt_commitment_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-selective-forecast-audit --input REGISTRATION.json [--source-commit SHA]"
            .to_owned()
    })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read selective forecast registration {input_path}: {error}"))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let receipt =
        validate_and_audit(&input, &source_commit).map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize selective forecast receipt: {error}"))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_slice(FIXTURE_BYTES).expect("synthetic fixture is valid JSON")
    }

    fn audit(value: &Value) -> Result<Value, Vec<String>> {
        validate_and_audit(
            &serde_json::to_vec(value).expect("test registration serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    #[test]
    fn fixture_reports_actual_coverage_and_conditional_risk_coverage() {
        let receipt = validate_and_audit(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("synthetic fixture contract validates");
        assert_eq!(receipt["total_record_count"], 12);
        assert_eq!(receipt["scored_record_count"], 7);
        assert!((receipt["actual_score_coverage"].as_f64().unwrap() - 7.0 / 12.0).abs() < 1e-12);
        let curve = receipt["risk_coverage_curve"].as_array().unwrap();
        assert_eq!(curve.len(), 4);
        assert_eq!(curve[0]["retained_count"], 2);
        assert!((curve[0]["brier_score"].as_f64().unwrap() - 0.04).abs() < 1e-12);
        assert_eq!(curve[1]["retained_count"], 4);
        assert!((curve[1]["brier_score"].as_f64().unwrap() - 0.0625).abs() < 1e-12);
        assert_eq!(curve[2]["retained_count"], 6);
        assert_eq!(curve[3]["retained_count"], 7);
        assert!((curve[3]["brier_score"].as_f64().unwrap() - 1.46 / 7.0).abs() < 1e-12);
        assert_eq!(receipt["claims"]["operational_utility_established"], false);
    }

    #[test]
    fn unknown_outcomes_are_not_imputed_and_remain_visible_by_disposition() {
        let receipt = validate_and_audit(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("synthetic fixture contract validates");
        assert_eq!(receipt["unknown_outcome_count"], 2);
        assert_eq!(receipt["metric_semantics"]["unknown_outcomes_imputed_to_no_event"], false);
        assert_eq!(receipt["metric_semantics"]["abstention_means_no_event"], false);
        let summaries = receipt["non_scored_disposition_summary"].as_array().unwrap();
        let stale = summaries
            .iter()
            .find(|summary| summary["disposition"] == "abstain_stale_inputs")
            .unwrap();
        assert_eq!(stale["event_count"], 1);
        assert_eq!(stale["no_event_count"], 0);
        let unknown = summaries
            .iter()
            .find(|summary| summary["disposition"] == "not_scored_unknown_outcome")
            .unwrap();
        assert_eq!(unknown["unknown_outcome_count"], 1);
    }

    #[test]
    fn fixed_input_replay_has_identical_receipt_commitment() {
        let first = validate_and_audit(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("first replay succeeds");
        let second = validate_and_audit(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("second replay succeeds");
        assert_eq!(first, second);
        assert_eq!(
            first["receipt_commitment_sha256"],
            second["receipt_commitment_sha256"]
        );
    }

    #[test]
    fn unknown_outcome_cannot_be_scored_as_a_no_event() {
        let mut value = fixture();
        value["records"][0]["outcome"] = json!("unknown");
        let errors = audit(&value).expect_err("unknown labels cannot be scored");
        assert!(errors.iter().any(|error| error.contains("unknown outcome")));
    }

    #[test]
    fn scored_record_cannot_have_stale_inputs() {
        let mut value = fixture();
        value["records"][0]["input_freshness"] = json!("stale");
        let errors = audit(&value).expect_err("stale inputs require explicit abstention");
        assert!(errors.iter().any(|error| error.contains("requires fresh inputs")));
    }

    #[test]
    fn abstention_cannot_publish_a_probability_or_selection_score() {
        let mut value = fixture();
        value["records"][4]["forecast_probability"] = json!(0.01);
        let errors = audit(&value).expect_err("abstained cases do not publish a scored forecast");
        assert!(errors.iter().any(|error| error.contains("must not publish")));
    }

    #[test]
    fn each_abstention_reason_requires_matching_evidence_state() {
        let mut value = fixture();
        value["records"][9]["evidence_status"] = json!("sufficient");
        let errors = audit(&value).expect_err("reason code must match the evidence state");
        assert!(errors.iter().any(|error| error.contains("without insufficient evidence")));
    }

    #[test]
    fn target_mismatch_cannot_pass_as_scored() {
        let mut value = fixture();
        value["records"][0]["target_matched"] = json!(false);
        let errors = audit(&value).expect_err("target mismatch requires abstention");
        assert!(errors.iter().any(|error| error.contains("must match the registered target")));
    }

    #[test]
    fn unknown_outcome_disposition_requires_an_unknown_label_and_clean_inputs() {
        let mut value = fixture();
        value["records"][5]["outcome"] = json!("no_event");
        let errors = audit(&value).expect_err("known labels cannot use unknown-outcome disposition");
        assert!(errors.iter().any(|error| error.contains("with a known outcome")));
    }

    #[test]
    fn duplicate_record_ids_are_rejected() {
        let mut value = fixture();
        let first_id = value["records"][0]["record_id"].clone();
        value["records"][1]["record_id"] = first_id;
        let errors = audit(&value).expect_err("duplicate row IDs must fail closed");
        assert!(errors.iter().any(|error| error.contains("record ID is duplicated")));
    }

    #[test]
    fn coverage_levels_must_be_ordered_and_end_at_full_coverage() {
        let mut value = fixture();
        value["preregistration"]["coverage_levels"] = json!([0.5, 0.25]);
        let errors = audit(&value).expect_err("coverage levels must be ordered and exhaustive");
        assert!(errors.iter().any(|error| error.contains("strictly increasing")));
        assert!(errors.iter().any(|error| error.contains("end at 1.0")));
    }

    #[test]
    fn small_groups_are_suppressed() {
        let mut value = fixture();
        value["privacy"]["group_min_count"] = json!(7);
        let receipt = audit(&value).expect("valid privacy policy still yields aggregate receipt");
        assert_eq!(receipt["group_metrics"].as_array().unwrap().len(), 0);
        assert_eq!(receipt["suppressed_group_count"], 2);
    }
}

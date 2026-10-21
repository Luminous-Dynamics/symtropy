// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic-only contract verifier for landscape model forecast accuracy.
//! This does not ingest field observations, establish causal effects, or qualify a physical model.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const FIXTURE_SCHEMA: &str = "luminous.landscape_forecast_verification_fixture.v0.1";
const FIXTURE_SUITE: &str = "LANDSCAPE-FORECAST-VERIFICATION-V0.1";
const CLAIM_CEILING: &str = "SYNTHETIC_CONTRACT_TEST_ONLY";
const MAX_ABS_VALUE: f64 = 1.0e12;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub schema_version: String,
    pub suite_id: String,
    pub data_origin: String,
    pub claim_ceiling: String,
    pub cases: Vec<EvaluationCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationCase {
    pub case_id: String,
    pub metric_id: String,
    pub unit: String,
    pub spatial_scope: String,
    pub temporal_stratum: String,
    pub prediction: Prediction,
    pub baseline: BaselineForecast,
    pub reference: ReferenceValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prediction {
    pub model_id: String,
    pub mean: f64,
    pub interval_level: f64,
    pub lower: f64,
    pub upper: f64,
    pub scope_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaselineForecast {
    pub model_id: String,
    pub mean: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceValue {
    pub status: ReferenceStatus,
    pub value: Option<f64>,
    pub uncertainty_abs: Option<f64>,
    pub method_id: String,
    pub scope_key: String,
    pub source_digest: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceStatus {
    Valid,
    Missing,
    Stale,
    Contradictory,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvaluationReport {
    pub schema_version: String,
    pub suite_id: String,
    pub input_sha256: String,
    pub data_origin: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub scored_case_count: usize,
    pub excluded_case_count: usize,
    pub scored_cases: Vec<ScoredCase>,
    pub excluded_cases: Vec<ExcludedCase>,
    pub metric_summaries: Vec<MetricSummary>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScoredCase {
    pub case_id: String,
    pub metric_id: String,
    pub unit: String,
    pub spatial_scope: String,
    pub temporal_stratum: String,
    pub model_id: String,
    pub baseline_model_id: String,
    pub reference_value: f64,
    pub reference_uncertainty_abs: f64,
    pub forecast_mean: f64,
    pub baseline_mean: f64,
    pub signed_error: f64,
    pub absolute_error: f64,
    pub prediction_interval_level: f64,
    pub prediction_interval_covered: bool,
    pub prediction_interval_width: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExcludedCase {
    pub case_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetricSummary {
    pub metric_id: String,
    pub unit: String,
    pub spatial_scope: String,
    pub temporal_stratum: String,
    pub model_id: String,
    pub baseline_model_id: String,
    pub prediction_interval_level: f64,
    pub sample_count: usize,
    pub mean_bias: f64,
    pub mae: f64,
    pub rmse: f64,
    pub baseline_mae: f64,
    pub mae_skill_score: Option<f64>,
    pub interval_covered_count: usize,
    pub interval_coverage: f64,
    pub mean_interval_width: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    metric_id: String,
    unit: String,
    spatial_scope: String,
    temporal_stratum: String,
    model_id: String,
    baseline_model_id: String,
    interval_level_micros: u64,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
}

fn bounded_finite(value: f64) -> bool {
    value.is_finite() && value.abs() <= MAX_ABS_VALUE
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

pub fn validate_fixture(fixture: &Fixture) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if fixture.schema_version != FIXTURE_SCHEMA {
        errors.push(format!(
            "unsupported schema_version: {}",
            fixture.schema_version
        ));
    }
    if fixture.suite_id != FIXTURE_SUITE {
        errors.push(format!("unexpected suite_id: {}", fixture.suite_id));
    }
    if fixture.data_origin != "synthetic_fixture" {
        errors.push("v0.1 accepts synthetic_fixture only; field observations need the qualified observation/evaluation path".to_owned());
    }
    if fixture.claim_ceiling != CLAIM_CEILING {
        errors.push(format!("claim_ceiling must be {CLAIM_CEILING}"));
    }
    if fixture.cases.is_empty() {
        errors.push("cases must not be empty".to_owned());
    }

    let mut ids = BTreeSet::new();
    for case in &fixture.cases {
        if !nonempty(&case.case_id) {
            errors.push("case_id must be non-empty".to_owned());
        }
        if !ids.insert(case.case_id.clone()) {
            errors.push(format!("duplicate case_id: {}", case.case_id));
        }
        for (name, value) in [
            ("metric_id", case.metric_id.as_str()),
            ("unit", case.unit.as_str()),
            ("spatial_scope", case.spatial_scope.as_str()),
            ("temporal_stratum", case.temporal_stratum.as_str()),
            ("prediction.model_id", case.prediction.model_id.as_str()),
            ("prediction.scope_key", case.prediction.scope_key.as_str()),
            ("baseline.model_id", case.baseline.model_id.as_str()),
            ("reference.method_id", case.reference.method_id.as_str()),
            ("reference.scope_key", case.reference.scope_key.as_str()),
        ] {
            if !nonempty(value) {
                errors.push(format!("{}: {name} must be non-empty", case.case_id));
            }
        }

        let p = &case.prediction;
        let expected_scope_key = format!("{}|{}", case.spatial_scope, case.temporal_stratum);
        if p.scope_key != expected_scope_key {
            errors.push(format!(
                "{}: prediction.scope_key must match declared spatial_scope and temporal_stratum ({expected_scope_key})",
                case.case_id
            ));
        }
        for (name, value) in [
            ("prediction.mean", p.mean),
            ("prediction.lower", p.lower),
            ("prediction.upper", p.upper),
            ("baseline.mean", case.baseline.mean),
        ] {
            if !bounded_finite(value) {
                errors.push(format!(
                    "{}: {name} must be finite and within ±{MAX_ABS_VALUE:e}",
                    case.case_id
                ));
            }
        }
        if !p.interval_level.is_finite() || !(0.0 < p.interval_level && p.interval_level < 1.0) {
            errors.push(format!(
                "{}: prediction.interval_level must be strictly between 0 and 1",
                case.case_id
            ));
        }
        if p.lower > p.mean || p.mean > p.upper {
            errors.push(format!(
                "{}: prediction interval must satisfy lower <= mean <= upper",
                case.case_id
            ));
        }

        let r = &case.reference;
        if let Some(digest) = &r.source_digest {
            if !valid_digest(digest) {
                errors.push(format!(
                    "{}: reference.source_digest must be 64 lowercase hexadecimal characters",
                    case.case_id
                ));
            }
        }
        match r.status {
            ReferenceStatus::Valid => {
                match r.value {
                    Some(value) if bounded_finite(value) => {}
                    _ => errors.push(format!(
                        "{}: valid reference requires a finite value",
                        case.case_id
                    )),
                }
                match r.uncertainty_abs {
                    Some(value) if bounded_finite(value) && value >= 0.0 => {}
                    _ => errors.push(format!(
                        "{}: valid reference requires non-negative finite uncertainty_abs",
                        case.case_id
                    )),
                }
                if r.source_digest
                    .as_deref()
                    .is_none_or(|digest| !valid_digest(digest))
                {
                    errors.push(format!("{}: valid reference requires a 64-character lowercase SHA-256 source_digest", case.case_id));
                }
            }
            ReferenceStatus::Missing | ReferenceStatus::Stale | ReferenceStatus::Contradictory => {
                if r.value.is_some() || r.uncertainty_abs.is_some() {
                    errors.push(format!(
                        "{}: non-valid reference status must not select a value or uncertainty",
                        case.case_id
                    ));
                }
                if r.status == ReferenceStatus::Missing && r.source_digest.is_some() {
                    errors.push(format!(
                        "{}: missing reference must not claim a source_digest",
                        case.case_id
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort();
        Err(errors)
    }
}

fn rounded(value: f64) -> f64 {
    let value = (value * 1.0e9).round() / 1.0e9;
    if value == 0.0 { 0.0 } else { value }
}

pub fn evaluate_fixture_bytes(bytes: &[u8]) -> Result<EvaluationReport, Vec<String>> {
    let fixture: Fixture = serde_json::from_slice(bytes)
        .map_err(|error| vec![format!("fixture JSON rejected: {error}")])?;
    validate_fixture(&fixture)?;
    evaluate_fixture(&fixture, bytes)
}

pub fn evaluate_fixture(
    fixture: &Fixture,
    input_bytes: &[u8],
) -> Result<EvaluationReport, Vec<String>> {
    validate_fixture(fixture)?;

    let mut scored_cases = Vec::new();
    let mut excluded_cases = Vec::new();
    let mut groups: BTreeMap<GroupKey, Vec<(String, f64, f64, f64, bool, f64, f64)>> =
        BTreeMap::new();

    for case in &fixture.cases {
        let r = &case.reference;
        match r.status {
            ReferenceStatus::Missing => {
                excluded_cases.push(ExcludedCase {
                    case_id: case.case_id.clone(),
                    reason: "missing_reference".to_owned(),
                });
                continue;
            }
            ReferenceStatus::Stale => {
                excluded_cases.push(ExcludedCase {
                    case_id: case.case_id.clone(),
                    reason: "stale_reference".to_owned(),
                });
                continue;
            }
            ReferenceStatus::Contradictory => {
                excluded_cases.push(ExcludedCase {
                    case_id: case.case_id.clone(),
                    reason: "contradictory_reference".to_owned(),
                });
                continue;
            }
            ReferenceStatus::Valid => {}
        }

        if case.prediction.scope_key != r.scope_key {
            excluded_cases.push(ExcludedCase {
                case_id: case.case_id.clone(),
                reason: "scope_mismatch".to_owned(),
            });
            continue;
        }

        let reference_value = r.value.expect("validated valid reference has a value");
        let reference_uncertainty_abs = r
            .uncertainty_abs
            .expect("validated valid reference has uncertainty");
        let prediction = &case.prediction;
        let baseline = &case.baseline;
        let signed_error = prediction.mean - reference_value;
        let absolute_error = signed_error.abs();
        let covered = prediction.lower <= reference_value && reference_value <= prediction.upper;
        let interval_width = prediction.upper - prediction.lower;
        let key = GroupKey {
            metric_id: case.metric_id.clone(),
            unit: case.unit.clone(),
            spatial_scope: case.spatial_scope.clone(),
            temporal_stratum: case.temporal_stratum.clone(),
            model_id: prediction.model_id.clone(),
            baseline_model_id: baseline.model_id.clone(),
            interval_level_micros: (prediction.interval_level * 1_000_000.0).round() as u64,
        };

        scored_cases.push(ScoredCase {
            case_id: case.case_id.clone(),
            metric_id: case.metric_id.clone(),
            unit: case.unit.clone(),
            spatial_scope: case.spatial_scope.clone(),
            temporal_stratum: case.temporal_stratum.clone(),
            model_id: prediction.model_id.clone(),
            baseline_model_id: baseline.model_id.clone(),
            reference_value,
            reference_uncertainty_abs,
            forecast_mean: prediction.mean,
            baseline_mean: baseline.mean,
            signed_error: rounded(signed_error),
            absolute_error: rounded(absolute_error),
            prediction_interval_level: prediction.interval_level,
            prediction_interval_covered: covered,
            prediction_interval_width: rounded(interval_width),
        });

        groups.entry(key).or_default().push((
            case.case_id.clone(),
            signed_error,
            absolute_error,
            signed_error * signed_error,
            covered,
            (baseline.mean - reference_value).abs(),
            interval_width,
        ));
    }

    scored_cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    excluded_cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));

    let mut metric_summaries = Vec::new();
    for (key, mut rows) in groups {
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        let n = rows.len() as f64;
        let mean_bias = rows.iter().map(|row| row.1).sum::<f64>() / n;
        let mae = rows.iter().map(|row| row.2).sum::<f64>() / n;
        let rmse = (rows.iter().map(|row| row.3).sum::<f64>() / n).sqrt();
        let baseline_mae = rows.iter().map(|row| row.5).sum::<f64>() / n;
        let covered_count = rows.iter().filter(|row| row.4).count();
        let interval_coverage = covered_count as f64 / n;
        let mean_interval_width = rows.iter().map(|row| row.6).sum::<f64>() / n;
        let interval_level = key.interval_level_micros as f64 / 1_000_000.0;

        metric_summaries.push(MetricSummary {
            metric_id: key.metric_id,
            unit: key.unit,
            spatial_scope: key.spatial_scope,
            temporal_stratum: key.temporal_stratum,
            model_id: key.model_id,
            baseline_model_id: key.baseline_model_id,
            prediction_interval_level: rounded(interval_level),
            sample_count: rows.len(),
            mean_bias: rounded(mean_bias),
            mae: rounded(mae),
            rmse: rounded(rmse),
            baseline_mae: rounded(baseline_mae),
            mae_skill_score: if baseline_mae > f64::EPSILON {
                Some(rounded(1.0 - mae / baseline_mae))
            } else {
                None
            },
            interval_covered_count: covered_count,
            interval_coverage: rounded(interval_coverage),
            mean_interval_width: rounded(mean_interval_width),
        });
    }

    Ok(EvaluationReport {
        schema_version: "luminous.landscape_forecast_verification_report.v0.1".to_owned(),
        suite_id: fixture.suite_id.clone(),
        input_sha256: sha256_hex(input_bytes),
        data_origin: fixture.data_origin.clone(),
        claim_ceiling: fixture.claim_ceiling.clone(),
        case_count: fixture.cases.len(),
        scored_case_count: scored_cases.len(),
        excluded_case_count: excluded_cases.len(),
        scored_cases,
        excluded_cases,
        metric_summaries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_BYTES: &[u8] =
        include_bytes!("../../../docs/research/fixtures/LANDSCAPE_FORECAST_VERIFICATION_V0_1.json");

    fn fixture() -> Fixture {
        serde_json::from_slice(FIXTURE_BYTES).expect("frozen fixture parses")
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-8,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn sha256_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn frozen_fixture_has_expected_disposition_counts() {
        let report = evaluate_fixture_bytes(FIXTURE_BYTES).expect("fixture is valid");
        assert_eq!(report.case_count, 13);
        assert_eq!(report.scored_case_count, 9);
        assert_eq!(report.excluded_case_count, 4);
        assert_eq!(report.metric_summaries.len(), 3);
        assert_eq!(report.data_origin, "synthetic_fixture");
        assert_eq!(report.claim_ceiling, CLAIM_CEILING);
        assert_eq!(report.input_sha256, sha256_hex(FIXTURE_BYTES));
        let reasons: BTreeSet<&str> = report
            .excluded_cases
            .iter()
            .map(|case| case.reason.as_str())
            .collect();
        assert_eq!(
            reasons,
            BTreeSet::from([
                "contradictory_reference",
                "missing_reference",
                "scope_mismatch",
                "stale_reference"
            ])
        );
    }

    #[test]
    fn grouped_error_metrics_match_independent_known_answers() {
        let report = evaluate_fixture_bytes(FIXTURE_BYTES).expect("fixture is valid");
        let heat = report
            .metric_summaries
            .iter()
            .find(|s| s.metric_id == "surface_temperature_delta")
            .unwrap();
        assert_eq!(heat.sample_count, 3);
        close(heat.mean_bias, 0.433333333);
        close(heat.mae, 0.7);
        close(heat.baseline_mae, 1.4);
        close(heat.mae_skill_score.unwrap(), 0.5);
        assert_eq!(heat.interval_covered_count, 2);
        close(heat.interval_coverage, 2.0 / 3.0);

        let water = report
            .metric_summaries
            .iter()
            .find(|s| s.metric_id == "runoff_volume_delta")
            .unwrap();
        assert_eq!(water.sample_count, 3);
        close(water.mae, 1.0 / 6.0);
        close(water.baseline_mae, 1.0 / 6.0);
        close(water.mae_skill_score.unwrap(), 0.0);

        let ecology = report
            .metric_summaries
            .iter()
            .find(|s| s.metric_id == "native_plant_survival_delta")
            .unwrap();
        assert_eq!(ecology.sample_count, 3);
        close(ecology.mae, 14.0 / 3.0);
        close(ecology.rmse, 6.0);
        close(ecology.baseline_mae, 26.0 / 3.0);
        close(ecology.mae_skill_score.unwrap(), 6.0 / 13.0);
    }

    #[test]
    fn no_cross_metric_scalar_is_emitted() {
        let report = evaluate_fixture_bytes(FIXTURE_BYTES).expect("fixture is valid");
        assert_eq!(report.metric_summaries.len(), 3);
        assert!(
            report
                .metric_summaries
                .iter()
                .all(|summary| summary.sample_count == 3)
        );
    }

    #[test]
    fn reversing_input_order_preserves_results_but_not_input_digest() {
        let baseline = evaluate_fixture_bytes(FIXTURE_BYTES).expect("fixture is valid");
        let mut reordered = fixture();
        reordered.cases.reverse();
        let bytes = serde_json::to_vec(&reordered).expect("fixture serializes");
        let report = evaluate_fixture_bytes(&bytes).expect("reordered fixture remains valid");
        assert_eq!(baseline.metric_summaries, report.metric_summaries);
        assert_eq!(baseline.scored_cases, report.scored_cases);
        assert_eq!(baseline.excluded_cases, report.excluded_cases);
        assert_ne!(baseline.input_sha256, report.input_sha256);
    }

    #[test]
    fn invalid_intervals_and_duplicate_ids_fail_closed() {
        let mut invalid_interval = fixture();
        invalid_interval.cases[0].prediction.lower = -1.0;
        assert!(validate_fixture(&invalid_interval).is_err());

        let mut wrong_scope = fixture();
        wrong_scope.cases[0].prediction.scope_key = "parcel|wet-season-2026".to_owned();
        assert!(validate_fixture(&wrong_scope).is_err());

        let mut duplicate = fixture();
        duplicate.cases[1].case_id = duplicate.cases[0].case_id.clone();
        assert!(validate_fixture(&duplicate).is_err());
    }

    #[test]
    fn field_data_cannot_bypass_the_v0_1_synthetic_claim_ceiling() {
        let mut field_labeled = fixture();
        field_labeled.data_origin = "field_observation".to_owned();
        field_labeled.claim_ceiling = "VALIDATED".to_owned();
        assert!(validate_fixture(&field_labeled).is_err());
    }

    #[test]
    fn invalid_digest_is_rejected() {
        let mut bad_digest = fixture();
        bad_digest.cases[0].reference.source_digest = Some("not-a-sha256".to_owned());
        assert!(validate_fixture(&bad_digest).is_err());
    }
}

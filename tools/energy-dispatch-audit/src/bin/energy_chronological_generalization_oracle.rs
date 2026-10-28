//! Chronological leakage and regime-shift qualification oracle.
//!
//! This frozen canary validates temporal evaluation rules. It is not a physical
//! dispatch simulator and its scores must not be represented as field performance.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs};

const PANEL_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-chronological-generalization-panel-v0.1.json");
const EXPERIMENT_BYTES: &[u8] =
    include_bytes!("../../fixtures/energy-chronological-generalization-experiment-v0.1.json");
const PANEL_SHA256: &str =
    "a2251addea0fa3f1694780f964ebb8b427d29b3bfa2b7d80278af17a6ebefbed";
const EXPERIMENT_SHA256: &str =
    "f2eb8561dfa65a84f08051bca1a24443c8ccdd18f654cf99138c5cd3d3205e24";
const PANEL_SCHEMA: &str = "energy-chronological-generalization-panel-v0.1";
const EXPERIMENT_SCHEMA: &str = "energy-chronological-generalization-experiment-v0.1";
const BENCHMARK_ID: &str = "energy-chronological-generalization-under-regime-shift-v0.1";

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Panel {
    schema_version: String,
    benchmark_id: String,
    claim_ceiling: String,
    split_contract: SplitContract,
    decision_threshold: f64,
    models: Vec<ModelSpec>,
    samples: Vec<Sample>,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SplitContract {
    train_indices: Vec<usize>,
    validation_indices: Vec<usize>,
    test_indices: Vec<usize>,
    segments_must_be_contiguous: bool,
    require_strictly_chronological_order: bool,
    purge_gap_for_one_step_horizon: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelSpec {
    model_id: String,
    rule: String,
    trained_through_index: usize,
    deployable: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sample {
    sample_id: String,
    time_index: usize,
    segment: String,
    regime: String,
    decision_time_index: i32,
    signal_value: f64,
    signal_available_at_index: i32,
    critical_event: u8,
    label_available_at_index: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Experiment {
    schema_version: String,
    benchmark_id: String,
    source_panel_path: String,
    source_panel_sha256: String,
    expected_coverage: ExpectedCoverage,
    known_answers: KnownAnswers,
    claim_ceiling: String,
    required_checks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedCoverage {
    sample_count: usize,
    train_count: usize,
    validation_count: usize,
    test_count: usize,
    model_count: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KnownAnswers {
    test_rows: usize,
    static_no_reserve: ModelAnswer,
    persistence: ModelAnswer,
    observable_signal: ModelAnswer,
    hindsight_oracle: HindsightAnswer,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelAnswer {
    false_negatives: u32,
    false_positives: u32,
    essential_unserved: u32,
    flexible_unserved: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HindsightAnswer {
    false_negatives: u32,
    false_positives: u32,
    deployable: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Confusion {
    true_positive: u32,
    true_negative: u32,
    false_positive: u32,
    false_negative: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct ModelResult {
    model_id: String,
    rule: String,
    deployable: bool,
    trained_through_index: usize,
    test_rows: usize,
    confusion: Confusion,
    essential_unserved: u32,
    flexible_unserved: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Report {
    schema_version: &'static str,
    benchmark_id: String,
    qualification_status: &'static str,
    claim_ceiling: String,
    source_panel_sha256: String,
    experiment_fixture_sha256: String,
    train_rows: usize,
    validation_rows: usize,
    test_rows: usize,
    model_results: BTreeMap<String, ModelResult>,
    ranked_deployable_models: Vec<String>,
    ranking_rule: &'static str,
    hindsight_reference_status: &'static str,
    physical_dispatch_audit: &'static str,
}

fn required_checks() -> [&'static str; 10] {
    [
        "raw_panel_sha256_matches_experiment_contract",
        "strict_time_order_and_unique_sample_ids",
        "train_validation_test_are_exact_nonoverlapping_contiguous_blocks",
        "features_are_available_at_or_before_decision_time",
        "labels_are_not_features",
        "deployable_models_do_not_train_past_train_cutoff",
        "hindsight_oracle_is_reported_but_excluded_from_deployable_rank",
        "test_metrics_use_held_out_test_rows_only",
        "service_metrics_are_vector_valued_not_aggregated_into_one_scalar",
        "deterministic_report_digest",
    ]
}

fn validate_panel(panel: &Panel) -> Result<(), String> {
    if panel.schema_version != PANEL_SCHEMA || panel.benchmark_id != BENCHMARK_ID {
        return Err("unsupported panel schema or benchmark identity".to_owned());
    }
    if !panel.claim_ceiling.to_ascii_lowercase().contains("synthetic")
        || !panel.claim_ceiling.to_ascii_lowercase().contains("only")
    {
        return Err("synthetic-only claim ceiling is mandatory".to_owned());
    }
    if !panel.decision_threshold.is_finite()
        || (panel.decision_threshold - 0.5).abs() > f64::EPSILON
    {
        return Err("decision threshold differs from the frozen 0.5 boundary".to_owned());
    }
    if panel.split_contract.train_indices != (0..=5).collect::<Vec<_>>()
        || panel.split_contract.validation_indices != (6..=8).collect::<Vec<_>>()
        || panel.split_contract.test_indices != (9..=14).collect::<Vec<_>>()
        || !panel.split_contract.segments_must_be_contiguous
        || !panel.split_contract.require_strictly_chronological_order
        || panel.split_contract.purge_gap_for_one_step_horizon != 0
    {
        return Err("split contract is not the frozen chronological block split".to_owned());
    }
    if panel.samples.len() != 15 {
        return Err("expected exactly 15 ordered samples".to_owned());
    }

    let mut sample_ids = BTreeSet::new();
    for (index, sample) in panel.samples.iter().enumerate() {
        if sample.time_index != index || !sample_ids.insert(sample.sample_id.as_str()) {
            return Err(format!("time order or unique sample identity violated at row {index}"));
        }
        let (expected_segment, expected_regime) = match index {
            0..=5 => ("train", "normal"),
            6..=8 => ("validation", "transition"),
            _ => ("test", "stress"),
        };
        if sample.segment != expected_segment || sample.regime != expected_regime {
            return Err(format!("split/regime ordering violated at row {index}"));
        }
        if sample.decision_time_index != index as i32 - 1 {
            return Err(format!("unexpected decision deadline at row {index}"));
        }
        if !sample.signal_value.is_finite() || !(0.0..=1.0).contains(&sample.signal_value) {
            return Err(format!("observable signal outside [0,1] at row {index}"));
        }
        if sample.signal_available_at_index > sample.decision_time_index {
            return Err(format!("future feature leakage at row {index}"));
        }
        if sample.label_available_at_index != sample.time_index
            || sample.label_available_at_index as i32 <= sample.decision_time_index
        {
            return Err(format!("target label is not strictly post-decision at row {index}"));
        }
        if sample.critical_event > 1 {
            return Err(format!("non-binary target label at row {index}"));
        }
        if index > 0
            && panel.samples[index - 1].label_available_at_index as i32
                > sample.decision_time_index
        {
            return Err(format!("prior label unavailable at decision boundary for row {index}"));
        }
    }

    let train_cutoff = *panel.split_contract.train_indices.last().unwrap();
    let expected_models = [
        ("static_no_reserve", "always_false", true),
        ("persistence", "previous_observed_label", true),
        ("observable_signal", "observable_signal_threshold", true),
        ("hindsight_oracle", "target_label", false),
    ];
    if panel.models.len() != expected_models.len() {
        return Err("model registry changed".to_owned());
    }
    let mut model_ids = BTreeSet::new();
    for (model, (expected_id, expected_rule, expected_deployable)) in
        panel.models.iter().zip(expected_models)
    {
        if model.model_id != expected_id
            || model.rule != expected_rule
            || model.deployable != expected_deployable
            || !model_ids.insert(model.model_id.as_str())
        {
            return Err(format!("model registry or deployability changed at {expected_id}"));
        }
        if model.deployable && model.trained_through_index > train_cutoff {
            return Err(format!("deployable model {} trained beyond train cutoff", model.model_id));
        }
        if model.rule == "target_label" && model.deployable {
            return Err("hindsight target-label rule cannot be deployable".to_owned());
        }
        if model.model_id == "hindsight_oracle" && model.trained_through_index != 14 {
            return Err("hindsight reference must be labelled with its full-panel access".to_owned());
        }
    }
    if panel.required_checks.len() != required_checks().len()
        || required_checks().iter().any(|required| !panel.required_checks.iter().any(|v| v == required))
    {
        return Err("required temporal-integrity check registry changed".to_owned());
    }
    Ok(())
}

fn validate_experiment(
    experiment: &Experiment,
    panel: &Panel,
    panel_bytes: &[u8],
) -> Result<(), String> {
    if experiment.schema_version != EXPERIMENT_SCHEMA || experiment.benchmark_id != BENCHMARK_ID {
        return Err("unsupported experiment schema or identity".to_owned());
    }
    if experiment.source_panel_path
        != "tools/energy-dispatch-audit/fixtures/energy-chronological-generalization-panel-v0.1.json"
    {
        return Err("source panel path mismatch".to_owned());
    }
    let actual_panel_sha = sha256_hex(panel_bytes);
    if actual_panel_sha != experiment.source_panel_sha256 {
        return Err(format!(
            "raw panel SHA-256 mismatch: expected {}, got {actual_panel_sha}",
            experiment.source_panel_sha256
        ));
    }
    if experiment.expected_coverage.sample_count != panel.samples.len()
        || experiment.expected_coverage.train_count != panel.samples.iter().filter(|s| s.segment == "train").count()
        || experiment.expected_coverage.validation_count != panel.samples.iter().filter(|s| s.segment == "validation").count()
        || experiment.expected_coverage.test_count != panel.samples.iter().filter(|s| s.segment == "test").count()
        || experiment.expected_coverage.model_count != panel.models.len()
        || experiment.known_answers.test_rows != experiment.expected_coverage.test_count
    {
        return Err("coverage contract changed".to_owned());
    }
    if !experiment.claim_ceiling.to_ascii_lowercase().contains("synthetic")
        || !experiment.claim_ceiling.to_ascii_lowercase().contains("not a")
        || experiment.required_checks.len() != required_checks().len()
        || required_checks().iter().any(|required| !experiment.required_checks.iter().any(|v| v == required))
    {
        return Err("experiment claim ceiling or integrity registry changed".to_owned());
    }
    Ok(())
}

fn predict(model: &ModelSpec, panel: &Panel, row_index: usize) -> Result<bool, String> {
    let row = panel.samples.get(row_index).ok_or_else(|| "sample index out of range".to_owned())?;
    match model.rule.as_str() {
        "always_false" => Ok(false),
        "previous_observed_label" => {
            if row_index == 0 {
                return Ok(false);
            }
            let previous = &panel.samples[row_index - 1];
            if previous.label_available_at_index as i32 > row.decision_time_index {
                return Err(format!("persistence uses unavailable label at row {row_index}"));
            }
            Ok(previous.critical_event == 1)
        }
        "observable_signal_threshold" => Ok(row.signal_value >= panel.decision_threshold),
        "target_label" if !model.deployable => Ok(row.critical_event == 1),
        "target_label" => Err("hindsight target-label access is non-deployable".to_owned()),
        other => Err(format!("unsupported model rule {other}")),
    }
}

fn test_metrics(panel: &Panel, model: &ModelSpec) -> Result<ModelResult, String> {
    let mut confusion = Confusion {
        true_positive: 0,
        true_negative: 0,
        false_positive: 0,
        false_negative: 0,
    };
    let mut rows = 0usize;
    for (index, sample) in panel.samples.iter().enumerate().filter(|(_, sample)| sample.segment == "test") {
        rows += 1;
        let predicted = predict(model, panel, index)?;
        match (predicted, sample.critical_event == 1) {
            (true, true) => confusion.true_positive += 1,
            (false, false) => confusion.true_negative += 1,
            (true, false) => confusion.false_positive += 1,
            (false, true) => confusion.false_negative += 1,
        }
    }
    Ok(ModelResult {
        model_id: model.model_id.clone(),
        rule: model.rule.clone(),
        deployable: model.deployable,
        trained_through_index: model.trained_through_index,
        test_rows: rows,
        essential_unserved: confusion.false_negative,
        flexible_unserved: confusion.false_positive,
        confusion,
    })
}

fn check_known_answers(
    experiment: &Experiment,
    results: &BTreeMap<String, ModelResult>,
) -> Result<(), String> {
    let checks = [
        ("static_no_reserve", &experiment.known_answers.static_no_reserve),
        ("persistence", &experiment.known_answers.persistence),
        ("observable_signal", &experiment.known_answers.observable_signal),
    ];
    for (model_id, expected) in checks {
        let actual = results.get(model_id).ok_or_else(|| format!("missing model result {model_id}"))?;
        if actual.confusion.false_negative != expected.false_negatives
            || actual.confusion.false_positive != expected.false_positives
            || actual.essential_unserved != expected.essential_unserved
            || actual.flexible_unserved != expected.flexible_unserved
        {
            return Err(format!("frozen known answer mismatch for {model_id}"));
        }
    }
    let oracle = results.get("hindsight_oracle").ok_or_else(|| "missing hindsight reference".to_owned())?;
    if oracle.confusion.false_negative != experiment.known_answers.hindsight_oracle.false_negatives
        || oracle.confusion.false_positive != experiment.known_answers.hindsight_oracle.false_positives
        || experiment.known_answers.hindsight_oracle.deployable
        || oracle.deployable
    {
        return Err("hindsight oracle known answer or non-deployable status changed".to_owned());
    }
    Ok(())
}

fn run_fixture() -> Result<Report, String> {
    if sha256_hex(PANEL_BYTES) != PANEL_SHA256 {
        return Err("pinned raw panel bytes have changed".to_owned());
    }
    if sha256_hex(EXPERIMENT_BYTES) != EXPERIMENT_SHA256 {
        return Err("pinned raw experiment bytes have changed".to_owned());
    }
    let panel: Panel = serde_json::from_slice(PANEL_BYTES)
        .map_err(|error| format!("panel parse failed: {error}"))?;
    let experiment: Experiment = serde_json::from_slice(EXPERIMENT_BYTES)
        .map_err(|error| format!("experiment parse failed: {error}"))?;
    validate_panel(&panel)?;
    validate_experiment(&experiment, &panel, PANEL_BYTES)?;

    let mut results = BTreeMap::new();
    for model in &panel.models {
        results.insert(model.model_id.clone(), test_metrics(&panel, model)?);
    }
    check_known_answers(&experiment, &results)?;

    let mut ranked: Vec<&ModelResult> = results.values().filter(|result| result.deployable).collect();
    ranked.sort_by_key(|result| (
        result.essential_unserved,
        result.flexible_unserved,
        result.model_id.as_str(),
    ));
    let ranked_deployable_models = ranked.iter().map(|result| result.model_id.clone()).collect();

    Ok(Report {
        schema_version: "energy-chronological-generalization-report-v0.1",
        benchmark_id: BENCHMARK_ID.to_owned(),
        qualification_status: "PASS_SYNTHETIC_TEMPORAL_INTEGRITY_ONLY",
        claim_ceiling: experiment.claim_ceiling,
        source_panel_sha256: sha256_hex(PANEL_BYTES),
        experiment_fixture_sha256: sha256_hex(EXPERIMENT_BYTES),
        train_rows: panel.samples.iter().filter(|s| s.segment == "train").count(),
        validation_rows: panel.samples.iter().filter(|s| s.segment == "validation").count(),
        test_rows: panel.samples.iter().filter(|s| s.segment == "test").count(),
        model_results: results,
        ranked_deployable_models,
        ranking_rule: "lexicographic: minimize essential-unserved events first, then flexible-unserved events; no scalar compensation",
        hindsight_reference_status: "NON_DEPLOYABLE_UPPER_BOUND_EXCLUDED_FROM_RANKING",
        physical_dispatch_audit: "NOT_RUN_SYNTHETIC_EVENT_CLASSIFICATION_ABSTRACTION",
    })
}

fn main() {
    let output_path = env::args().nth(1);
    match run_fixture() {
        Ok(report) => {
            let serialized = match serde_json::to_vec_pretty(&report) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("FAIL report serialization: {error}");
                    std::process::exit(2);
                }
            };
            if let Some(path) = output_path {
                if let Err(error) = fs::write(&path, &serialized) {
                    eprintln!("FAIL writing report {}: {}", path, error);
                    std::process::exit(2);
                }
            } else {
                println!("{}", String::from_utf8_lossy(&serialized));
            }
            println!("PASS chronological generalization protocol canary");
            println!("panel SHA-256: {}", report.source_panel_sha256);
            println!("experiment SHA-256: {}", report.experiment_fixture_sha256);
            println!("test rows: {}", report.test_rows);
            println!("deployable ranking: {}", report.ranked_deployable_models.join(" < "));
            println!("physical dispatch audit: {}", report.physical_dispatch_audit);
        }
        Err(error) => {
            eprintln!("FAIL chronological generalization protocol canary: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Panel {
        serde_json::from_slice(PANEL_BYTES).unwrap()
    }

    #[test]
    fn frozen_known_answers_and_ranking_replay() {
        let report = run_fixture().unwrap();
        assert_eq!(report.test_rows, 6);
        assert_eq!(report.ranked_deployable_models, vec![
            "observable_signal", "persistence", "static_no_reserve"
        ]);
        assert_eq!(report.model_results["static_no_reserve"].essential_unserved, 4);
        assert_eq!(report.model_results["static_no_reserve"].flexible_unserved, 0);
        assert_eq!(report.model_results["persistence"].essential_unserved, 2);
        assert_eq!(report.model_results["persistence"].flexible_unserved, 1);
        assert_eq!(report.model_results["observable_signal"].essential_unserved, 0);
        assert_eq!(report.model_results["observable_signal"].flexible_unserved, 1);
        assert!(!report.model_results["hindsight_oracle"].deployable);
        assert_eq!(report.physical_dispatch_audit, "NOT_RUN_SYNTHETIC_EVENT_CLASSIFICATION_ABSTRACTION");
    }

    #[test]
    fn rejects_future_feature_availability() {
        let mut p = panel();
        p.samples[10].signal_available_at_index = 10;
        assert!(validate_panel(&p).unwrap_err().contains("future feature leakage"));
    }

    #[test]
    fn rejects_nonchronological_or_randomized_split() {
        let mut p = panel();
        p.samples.swap(6, 9);
        assert!(validate_panel(&p).is_err());
        let mut p = panel();
        p.samples[9].segment = "train".to_owned();
        assert!(validate_panel(&p).is_err());
    }

    #[test]
    fn rejects_training_past_the_train_cutoff() {
        let mut p = panel();
        p.models.iter_mut().find(|m| m.model_id == "persistence").unwrap().trained_through_index = 6;
        assert!(validate_panel(&p).unwrap_err().contains("trained beyond train cutoff"));
    }

    #[test]
    fn rejects_deployable_hindsight_oracle() {
        let mut p = panel();
        p.models.iter_mut().find(|m| m.model_id == "hindsight_oracle").unwrap().deployable = true;
        assert!(validate_panel(&p).is_err());
    }

    #[test]
    fn rejects_a_feature_arriving_after_the_label_deadline() {
        let mut p = panel();
        p.samples[12].signal_available_at_index = p.samples[12].time_index as i32;
        assert!(validate_panel(&p).is_err());
    }

    #[test]
    fn train_label_changes_do_not_enter_test_metrics() {
        let p = panel();
        let mut altered = panel();
        altered.samples[0].critical_event = 1;
        let base_model = p.models.iter().find(|m| m.model_id == "static_no_reserve").unwrap();
        let altered_model = altered.models.iter().find(|m| m.model_id == "static_no_reserve").unwrap();
        assert_eq!(test_metrics(&p, base_model).unwrap(), test_metrics(&altered, altered_model).unwrap());
        let base_model = p.models.iter().find(|m| m.model_id == "persistence").unwrap();
        let altered_model = altered.models.iter().find(|m| m.model_id == "persistence").unwrap();
        assert_eq!(test_metrics(&p, base_model).unwrap(), test_metrics(&altered, altered_model).unwrap());
    }

    #[test]
    fn raw_source_digest_tampering_is_detected() {
        let panel: Panel = serde_json::from_slice(PANEL_BYTES).unwrap();
        let experiment: Experiment = serde_json::from_slice(EXPERIMENT_BYTES).unwrap();
        assert!(validate_experiment(&experiment, &panel, b"{}").unwrap_err().contains("raw panel SHA-256 mismatch"));
    }

    #[test]
    fn repeated_reports_are_byte_deterministic() {
        let first = serde_json::to_vec(&run_fixture().unwrap()).unwrap();
        let second = serde_json::to_vec(&run_fixture().unwrap()).unwrap();
        assert_eq!(first, second);
    }
}

// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Episode-level split-conformal threshold and set filtering for WPT pose hypotheses.
//! Scores, metric definitions, and shift dispositions are supplied by upstream owners.

use std::collections::BTreeSet;
use std::{env, fs, process::ExitCode};

const CALIBRATION_HEADER: [&str; 13] = [
    "calibrator_id",
    "evidence_class",
    "stratum_id",
    "sampling_unit",
    "partition",
    "alpha_ppm",
    "metric_sha256",
    "predictor_sha256",
    "hardware_sha256",
    "environment_sha256",
    "calibration_episode_id",
    "score_micros",
    "valid_until_us",
];

const INFERENCE_HEADER: [&str; 19] = [
    "inference_id",
    "calibrator_id",
    "evidence_class",
    "stratum_id",
    "sampling_unit",
    "partition",
    "decision_time_us",
    "alpha_ppm",
    "metric_sha256",
    "predictor_sha256",
    "hardware_sha256",
    "environment_sha256",
    "shift_status",
    "observation_sha256",
    "pose_id",
    "model_id",
    "fault_id",
    "score_micros",
    "valid_until_us",
];

const MAX_CALIBRATION_EPISODES: usize = 1_000_000;
const MAX_INFERENCE_HYPOTHESES: usize = 16_384;
const PPM: u128 = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Calibration {
    calibrator_id: String,
    evidence_class: String,
    stratum_id: String,
    alpha_ppm: u32,
    metric_sha256: String,
    predictor_sha256: String,
    hardware_sha256: String,
    environment_sha256: String,
    scores_micros: Vec<u64>,
    minimum_valid_until_us: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Hypothesis {
    pose_id: String,
    model_id: String,
    fault_id: String,
    score_micros: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Inference {
    inference_id: String,
    calibrator_id: String,
    evidence_class: String,
    stratum_id: String,
    decision_time_us: u64,
    alpha_ppm: u32,
    metric_sha256: String,
    predictor_sha256: String,
    hardware_sha256: String,
    environment_sha256: String,
    shift_status: String,
    observation_sha256: String,
    valid_until_us: u64,
    hypotheses: Vec<Hypothesis>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evaluation {
    disposition: &'static str,
    calibration_episode_count: usize,
    quantile_rank: Option<usize>,
    threshold_micros: Option<u64>,
    target_coverage_ppm: u32,
    surviving_pose_ids: BTreeSet<String>,
    surviving_model_ids: BTreeSet<String>,
    surviving_fault_ids: BTreeSet<String>,
    surviving_hypothesis_count: usize,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!("{field}: empty or untrimmed integer"));
    }
    value
        .parse::<u64>()
        .map_err(|_| format!("{field}: expected non-negative integer"))
}

fn parse_alpha(value: &str, field: &str) -> Result<u32, String> {
    let parsed = parse_u64(value, field)?;
    if parsed == 0 || parsed >= PPM as u64 {
        return Err(format!("{field}: alpha_ppm must be in 1..999999"));
    }
    u32::try_from(parsed).map_err(|_| format!("{field}: alpha_ppm is out of range"))
}

fn supported_evidence_class(value: &str) -> bool {
    matches!(
        value,
        "SYNTHETIC_KNOWN_TRUTH" | "SIMULATED" | "BENCH_MEASURED" | "FIELD_MEASURED"
    )
}

fn parse_calibration(input: &str) -> Result<Calibration, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["calibration input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != CALIBRATION_HEADER.as_slice() {
        return Err(vec![format!(
            "calibration header must contain exactly {} columns",
            CALIBRATION_HEADER.len()
        )]);
    }

    let mut metadata: Option<Vec<String>> = None;
    let mut calibrator_id = String::new();
    let mut evidence_class = String::new();
    let mut stratum_id = String::new();
    let mut alpha_ppm = 0u32;
    let mut metric_sha256 = String::new();
    let mut predictor_sha256 = String::new();
    let mut hardware_sha256 = String::new();
    let mut environment_sha256 = String::new();
    let mut episode_ids = BTreeSet::new();
    let mut scores_micros = Vec::new();
    let mut minimum_valid_until_us = u64::MAX;
    let mut errors = Vec::new();

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("calibration line {line_no}: blank rows are not allowed"));
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != CALIBRATION_HEADER.len() {
            errors.push(format!(
                "calibration line {line_no}: expected {} columns, found {}",
                CALIBRATION_HEADER.len(),
                fields.len()
            ));
            continue;
        }

        let row_metadata: Vec<String> =
            fields[..10].iter().map(|field| (*field).to_string()).collect();
        if let Some(first) = &metadata {
            if *first != row_metadata {
                errors.push(format!(
                    "calibration line {line_no}: calibrator metadata differs from first row"
                ));
                continue;
            }
        } else {
            metadata = Some(row_metadata);
            calibrator_id = fields[0].to_string();
            evidence_class = fields[1].to_string();
            stratum_id = fields[2].to_string();
            let parsed_alpha = match parse_alpha(fields[5], "alpha_ppm") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(format!("calibration line {line_no}: {error}"));
                    continue;
                }
            };
            alpha_ppm = parsed_alpha;
            metric_sha256 = fields[6].to_string();
            predictor_sha256 = fields[7].to_string();
            hardware_sha256 = fields[8].to_string();
            environment_sha256 = fields[9].to_string();

            if !valid_identifier(&calibrator_id)
                || !supported_evidence_class(&evidence_class)
                || !valid_identifier(&stratum_id)
                || fields[3] != "INDEPENDENT_EPISODE"
                || fields[4] != "CALIBRATION"
                || !digest_valid(&metric_sha256)
                || !digest_valid(&predictor_sha256)
                || !digest_valid(&hardware_sha256)
                || !digest_valid(&environment_sha256)
            {
                errors.push(format!(
                    "calibration line {line_no}: invalid calibrator metadata, sampling unit, partition or digest"
                ));
                continue;
            }
        }

        if !valid_identifier(fields[10]) {
            errors.push(format!(
                "calibration line {line_no}: invalid calibration_episode_id"
            ));
            continue;
        }
        if !episode_ids.insert(fields[10].to_string()) {
            errors.push(format!(
                "calibration line {line_no}: duplicate calibration_episode_id {}",
                fields[10]
            ));
            continue;
        }

        let score_micros = match parse_u64(fields[11], "score_micros") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("calibration line {line_no}: {error}"));
                continue;
            }
        };
        let valid_until_us = match parse_u64(fields[12], "valid_until_us") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("calibration line {line_no}: {error}"));
                continue;
            }
        };

        scores_micros.push(score_micros);
        minimum_valid_until_us = minimum_valid_until_us.min(valid_until_us);
        if scores_micros.len() > MAX_CALIBRATION_EPISODES {
            errors.push(format!(
                "calibration case exceeds {MAX_CALIBRATION_EPISODES} independent episodes"
            ));
            break;
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    if scores_micros.is_empty() {
        return Err(vec!["calibration file has no independent episode scores".to_string()]);
    }
    scores_micros.sort_unstable();

    Ok(Calibration {
        calibrator_id,
        evidence_class,
        stratum_id,
        alpha_ppm,
        metric_sha256,
        predictor_sha256,
        hardware_sha256,
        environment_sha256,
        scores_micros,
        minimum_valid_until_us,
    })
}

fn parse_inference(input: &str) -> Result<Inference, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["inference input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != INFERENCE_HEADER.as_slice() {
        return Err(vec![format!(
            "inference header must contain exactly {} columns",
            INFERENCE_HEADER.len()
        )]);
    }

    let mut metadata: Option<Vec<String>> = None;
    let mut inference_id = String::new();
    let mut calibrator_id = String::new();
    let mut evidence_class = String::new();
    let mut stratum_id = String::new();
    let mut decision_time_us = 0u64;
    let mut alpha_ppm = 0u32;
    let mut metric_sha256 = String::new();
    let mut predictor_sha256 = String::new();
    let mut hardware_sha256 = String::new();
    let mut environment_sha256 = String::new();
    let mut shift_status = String::new();
    let mut observation_sha256 = String::new();
    let mut valid_until_us = u64::MAX;
    let mut tuple_ids = BTreeSet::new();
    let mut hypotheses = Vec::new();
    let mut errors = Vec::new();

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("inference line {line_no}: blank rows are not allowed"));
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != INFERENCE_HEADER.len() {
            errors.push(format!(
                "inference line {line_no}: expected {} columns, found {}",
                INFERENCE_HEADER.len(),
                fields.len()
            ));
            continue;
        }

        let mut row_metadata: Vec<String> =
            fields[..14].iter().map(|field| (*field).to_string()).collect();
        row_metadata.push(fields[18].to_string());
        if let Some(first) = &metadata {
            if *first != row_metadata {
                errors.push(format!(
                    "inference line {line_no}: episode metadata differs from first hypothesis"
                ));
                continue;
            }
        } else {
            metadata = Some(row_metadata);
            inference_id = fields[0].to_string();
            calibrator_id = fields[1].to_string();
            evidence_class = fields[2].to_string();
            stratum_id = fields[3].to_string();
            let parsed_decision_time = match parse_u64(fields[6], "decision_time_us") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(format!("inference line {line_no}: {error}"));
                    continue;
                }
            };
            decision_time_us = parsed_decision_time;
            alpha_ppm = match parse_alpha(fields[7], "alpha_ppm") {
                Ok(value) => value,
                Err(error) => {
                    errors.push(format!("inference line {line_no}: {error}"));
                    continue;
                }
            };
            metric_sha256 = fields[8].to_string();
            predictor_sha256 = fields[9].to_string();
            hardware_sha256 = fields[10].to_string();
            environment_sha256 = fields[11].to_string();
            shift_status = fields[12].to_string();
            observation_sha256 = fields[13].to_string();

            if !valid_identifier(&inference_id)
                || !valid_identifier(&calibrator_id)
                || !supported_evidence_class(&evidence_class)
                || !valid_identifier(&stratum_id)
                || fields[4] != "INDEPENDENT_EPISODE"
                || fields[5] != "TEST"
                || !matches!(shift_status.as_str(), "SUPPORTED" | "SHIFT_DETECTED" | "UNKNOWN")
                || !digest_valid(&metric_sha256)
                || !digest_valid(&predictor_sha256)
                || !digest_valid(&hardware_sha256)
                || !digest_valid(&environment_sha256)
                || !digest_valid(&observation_sha256)
            {
                errors.push(format!(
                    "inference line {line_no}: invalid inference metadata, sampling unit, partition, shift status or digest"
                ));
                continue;
            }
        }

        let pose_id = fields[14];
        let model_id = fields[15];
        let fault_id = fields[16];
        if !valid_identifier(pose_id)
            || !valid_identifier(model_id)
            || !valid_identifier(fault_id)
        {
            errors.push(format!(
                "inference line {line_no}: invalid pose, model or fault identifier"
            ));
            continue;
        }
        let score_micros = match parse_u64(fields[17], "score_micros") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("inference line {line_no}: {error}"));
                continue;
            }
        };
        let row_valid_until_us = match parse_u64(fields[18], "valid_until_us") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("inference line {line_no}: {error}"));
                continue;
            }
        };
        if row_valid_until_us < decision_time_us {
            errors.push(format!(
                "inference line {line_no}: candidate evidence expired before decision time"
            ));
            continue;
        }
        valid_until_us = valid_until_us.min(row_valid_until_us);

        let tuple_key = format!("{pose_id}|{model_id}|{fault_id}");
        if !tuple_ids.insert(tuple_key) {
            errors.push(format!(
                "inference line {line_no}: duplicate pose/model/fault hypothesis tuple"
            ));
            continue;
        }
        hypotheses.push(Hypothesis {
            pose_id: pose_id.to_string(),
            model_id: model_id.to_string(),
            fault_id: fault_id.to_string(),
            score_micros,
        });
        if hypotheses.len() > MAX_INFERENCE_HYPOTHESES {
            errors.push(format!(
                "inference case exceeds {MAX_INFERENCE_HYPOTHESES} hypotheses"
            ));
            break;
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    if hypotheses.is_empty() {
        return Err(vec!["inference file has no candidate hypotheses".to_string()]);
    }
    if valid_until_us < decision_time_us {
        return Err(vec!["inference evidence is stale at decision time".to_string()]);
    }

    hypotheses.sort();
    Ok(Inference {
        inference_id,
        calibrator_id,
        evidence_class,
        stratum_id,
        decision_time_us,
        alpha_ppm,
        metric_sha256,
        predictor_sha256,
        hardware_sha256,
        environment_sha256,
        shift_status,
        observation_sha256,
        valid_until_us,
        hypotheses,
    })
}

fn validate_join(calibration: &Calibration, inference: &Inference) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if calibration.calibrator_id != inference.calibrator_id {
        errors.push("inference references a different calibrator_id".to_string());
    }
    if calibration.evidence_class != inference.evidence_class {
        errors.push("calibration and inference evidence classes differ".to_string());
    }
    if calibration.stratum_id != inference.stratum_id {
        errors.push("cross-stratum calibration is prohibited".to_string());
    }
    if calibration.alpha_ppm != inference.alpha_ppm {
        errors.push("calibration and inference alpha_ppm differ".to_string());
    }
    if calibration.metric_sha256 != inference.metric_sha256 {
        errors.push("joint nonconformity metric digest mismatch".to_string());
    }
    if calibration.predictor_sha256 != inference.predictor_sha256 {
        errors.push("predictor digest mismatch".to_string());
    }
    if calibration.hardware_sha256 != inference.hardware_sha256 {
        errors.push("receiver hardware digest mismatch".to_string());
    }
    if calibration.environment_sha256 != inference.environment_sha256 {
        errors.push("environment stratum digest mismatch".to_string());
    }
    if inference.decision_time_us > calibration.minimum_valid_until_us {
        errors.push("calibration evidence expired before inference decision time".to_string());
    }
    if inference.decision_time_us > inference.valid_until_us {
        errors.push("inference candidate support expired before decision time".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Non-randomized split-conformal rank, one-indexed.
/// Returns None when no finite order statistic exists (rank > n).
fn conformal_rank(sample_count: usize, alpha_ppm: u32) -> Option<usize> {
    if sample_count == 0 || u128::from(alpha_ppm) == 0 || u128::from(alpha_ppm) >= PPM {
        return None;
    }
    let numerator = (sample_count as u128 + 1) * (PPM - u128::from(alpha_ppm));
    let rank = numerator.div_ceil(PPM);
    if rank > sample_count as u128 {
        None
    } else {
        usize::try_from(rank).ok()
    }
}

fn evaluate(calibration: &Calibration, inference: &Inference) -> Evaluation {
    let target_coverage_ppm = 1_000_000 - calibration.alpha_ppm;
    let rank = conformal_rank(calibration.scores_micros.len(), calibration.alpha_ppm);
    let threshold = rank.map(|value| calibration.scores_micros[value - 1]);

    let mut result = Evaluation {
        disposition: "NO_QUALIFIED_DISPOSITION",
        calibration_episode_count: calibration.scores_micros.len(),
        quantile_rank: rank,
        threshold_micros: threshold,
        target_coverage_ppm,
        surviving_pose_ids: BTreeSet::new(),
        surviving_model_ids: BTreeSet::new(),
        surviving_fault_ids: BTreeSet::new(),
        surviving_hypothesis_count: 0,
    };

    if inference.shift_status == "SHIFT_DETECTED" {
        result.disposition = "ABSTAIN_SHIFT_DETECTED";
        return result;
    }
    if inference.shift_status == "UNKNOWN" {
        result.disposition = "ABSTAIN_SHIFT_UNKNOWN";
        return result;
    }
    let Some(threshold) = threshold else {
        result.disposition = "ABSTAIN_INSUFFICIENT_CALIBRATION";
        return result;
    };

    for hypothesis in &inference.hypotheses {
        if hypothesis.score_micros <= threshold {
            result.surviving_pose_ids.insert(hypothesis.pose_id.clone());
            result.surviving_model_ids.insert(hypothesis.model_id.clone());
            result.surviving_fault_ids.insert(hypothesis.fault_id.clone());
            result.surviving_hypothesis_count += 1;
        }
    }

    result.disposition = match result.surviving_pose_ids.len() {
        0 => "EMPTY_CALIBRATED_SET",
        1 => "CALIBRATED_SINGLE_POSE_SET_MARGINAL_COVERAGE_ONLY",
        _ => "CALIBRATED_MULTI_POSE_SET",
    };
    result
}

fn run(calibration_path: &str, inference_path: &str) -> Result<(), String> {
    let calibration_text = fs::read_to_string(calibration_path)
        .map_err(|error| format!("cannot read {calibration_path}: {error}"))?;
    let inference_text = fs::read_to_string(inference_path)
        .map_err(|error| format!("cannot read {inference_path}: {error}"))?;
    let calibration =
        parse_calibration(&calibration_text).map_err(|errors| errors.join("\n"))?;
    let inference = parse_inference(&inference_text).map_err(|errors| errors.join("\n"))?;
    validate_join(&calibration, &inference).map_err(|errors| errors.join("\n"))?;
    let result = evaluate(&calibration, &inference);

    println!("inference_id={}", inference.inference_id);
    println!("calibrator_id={}", calibration.calibrator_id);
    println!("evidence_class={}", calibration.evidence_class);
    println!("stratum_id={}", calibration.stratum_id);
    println!("calibration_episode_count={}", result.calibration_episode_count);
    println!("alpha_ppm={}", calibration.alpha_ppm);
    println!("target_marginal_coverage_ppm={}", result.target_coverage_ppm);
    match result.quantile_rank {
        Some(rank) => println!("quantile_rank={rank}"),
        None => println!("quantile_rank=UNAVAILABLE"),
    }
    match result.threshold_micros {
        Some(threshold) => println!("threshold_micros={threshold}"),
        None => println!("threshold_micros=UNBOUNDED_ABSTAIN"),
    }
    println!("shift_status={}", inference.shift_status);
    println!("disposition={}", result.disposition);
    println!(
        "surviving_pose_ids={}",
        result
            .surviving_pose_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(";")
    );
    println!(
        "surviving_model_ids={}",
        result
            .surviving_model_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(";")
    );
    println!(
        "surviving_fault_ids={}",
        result
            .surviving_fault_ids
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(";")
    );
    println!("surviving_hypothesis_count={}", result.surviving_hypothesis_count);
    println!("coverage_interpretation=MARGINAL_ONLY_UNDER_EPISODE_EXCHANGEABILITY");
    println!("conditional_singleton_correctness=NOT_ESTABLISHED");
    println!("shift_detection=UPSTREAM_SUPPLIED_NOT_IMPLEMENTED_HERE");
    println!("operational_permit=false");
    println!("authority=NONE_CALIBRATION_SET_EVALUATION_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: wpt-pose-calibration <calibration.tsv> <inference.tsv>");
        return ExitCode::from(2);
    }
    match run(&args[1], &args[2]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("REJECTED_WPT_POSE_CALIBRATION {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CALIBRATION: &str =
        include_str!("../../../research/wpt-pose-calibration-v0/v-guide-calibration.tsv");
    const INFERENCE: &str =
        include_str!("../../../research/wpt-pose-calibration-v0/v-guide-inference.tsv");
    const SHIFTED: &str =
        include_str!("../../../research/wpt-pose-calibration-v0/shift-detected-inference.tsv");

    fn parsed_calibration(input: &str) -> Calibration {
        parse_calibration(input).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    fn parsed_inference(input: &str) -> Inference {
        parse_inference(input).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    fn composed(calibration_text: &str, inference_text: &str) -> (Calibration, Inference, Evaluation) {
        let calibration = parsed_calibration(calibration_text);
        let inference = parsed_inference(inference_text);
        validate_join(&calibration, &inference)
            .unwrap_or_else(|errors| panic!("{}", errors.join("\n")));
        let result = evaluate(&calibration, &inference);
        (calibration, inference, result)
    }

    #[test]
    fn nine_episode_alpha_ten_percent_uses_ninth_order_statistic() {
        let (calibration, inference, result) = composed(CALIBRATION, INFERENCE);
        assert_eq!(calibration.scores_micros.len(), 9);
        assert_eq!(result.quantile_rank, Some(9));
        assert_eq!(result.threshold_micros, Some(90));
        assert_eq!(result.target_coverage_ppm, 900_000);
        assert_eq!(
            result.disposition,
            "CALIBRATED_SINGLE_POSE_SET_MARGINAL_COVERAGE_ONLY"
        );
        assert_eq!(result.surviving_pose_ids, BTreeSet::from(["POSE_A".to_string()]));
        assert_eq!(result.surviving_hypothesis_count, 2);
        assert_eq!(result.surviving_model_ids.len(), 2);
        assert_eq!(result.surviving_fault_ids.len(), 2);
        assert_eq!(inference.shift_status, "SUPPORTED");
    }

    #[test]
    fn score_equal_to_threshold_is_included() {
        let inference = INFERENCE.replace("\t95\t1000", "\t90\t1000");
        let (_, _, result) = composed(CALIBRATION, &inference);
        assert_eq!(result.disposition, "CALIBRATED_MULTI_POSE_SET");
        assert!(result.surviving_pose_ids.contains("POSE_B"));
    }

    #[test]
    fn insufficient_samples_never_fall_back_to_the_maximum_score() {
        let eight = CALIBRATION
            .lines()
            .take(9)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let calibration = parsed_calibration(&eight);
        let inference = parsed_inference(INFERENCE);
        validate_join(&calibration, &inference).expect("matching calibrator");
        let result = evaluate(&calibration, &inference);
        assert_eq!(result.quantile_rank, None);
        assert_eq!(result.threshold_micros, None);
        assert_eq!(result.disposition, "ABSTAIN_INSUFFICIENT_CALIBRATION");
        assert!(result.surviving_pose_ids.is_empty());
    }

    #[test]
    fn high_coverage_sample_size_boundaries_are_exact() {
        assert_eq!(conformal_rank(998, 1_000), None);
        assert_eq!(conformal_rank(999, 1_000), Some(999));
        assert_eq!(conformal_rank(9_998, 100), None);
        assert_eq!(conformal_rank(9_999, 100), Some(9_999));
    }

    #[test]
    fn shifted_stratum_abstains_even_when_a_finite_threshold_exists() {
        let (calibration, inference, result) = composed(CALIBRATION, SHIFTED);
        assert_eq!(calibration.scores_micros.len(), 9);
        assert_eq!(result.threshold_micros, Some(90));
        assert_eq!(inference.shift_status, "SHIFT_DETECTED");
        assert_eq!(result.disposition, "ABSTAIN_SHIFT_DETECTED");
        assert!(result.surviving_pose_ids.is_empty());
    }

    #[test]
    fn unknown_shift_status_abstains() {
        let unknown = SHIFTED.replace("SHIFT_DETECTED", "UNKNOWN");
        let (_, _, result) = composed(CALIBRATION, &unknown);
        assert_eq!(result.disposition, "ABSTAIN_SHIFT_UNKNOWN");
    }

    #[test]
    fn all_candidates_above_threshold_produce_empty_set_not_fake_uniqueness() {
        let outside = INFERENCE
            .replace("\t50\t1000", "\t100\t1000")
            .replace("\t90\t1000", "\t101\t1000")
            .replace("\t95\t1000", "\t102\t1000");
        let (_, _, result) = composed(CALIBRATION, &outside);
        assert_eq!(result.disposition, "EMPTY_CALIBRATED_SET");
        assert!(result.surviving_pose_ids.is_empty());
    }

    #[test]
    fn cross_stratum_calibration_is_rejected() {
        let other_stratum = INFERENCE.replace("POST_CONTACT_V_GUIDE", "FLAT_PAD");
        let calibration = parsed_calibration(CALIBRATION);
        let inference = parsed_inference(&other_stratum);
        assert!(validate_join(&calibration, &inference).is_err());
    }

    #[test]
    fn predictor_digest_mismatch_is_rejected() {
        let other_predictor = INFERENCE.replace(
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let calibration = parsed_calibration(CALIBRATION);
        let inference = parsed_inference(&other_predictor);
        assert!(validate_join(&calibration, &inference).is_err());
    }

    #[test]
    fn calibration_expiry_invalidates_the_whole_calibrator() {
        let expired = CALIBRATION.replace("\t10000", "\t99");
        let calibration = parsed_calibration(&expired);
        let inference = parsed_inference(INFERENCE);
        assert!(validate_join(&calibration, &inference).is_err());
    }

    #[test]
    fn calibration_episodes_must_be_unique() {
        let duplicate = format!(
            "{CALIBRATION}{}",
            CALIBRATION.lines().nth(1).expect("first calibration episode")
        );
        assert!(parse_calibration(&duplicate).is_err());
    }

    #[test]
    fn frame_level_sampling_units_are_rejected() {
        let frame = CALIBRATION.replace("INDEPENDENT_EPISODE", "FRAME");
        assert!(parse_calibration(&frame).is_err());
    }

    #[test]
    fn duplicate_inference_hypothesis_tuples_are_rejected() {
        let duplicate = format!(
            "{INFERENCE}{}",
            INFERENCE.lines().nth(1).expect("first candidate")
        );
        assert!(parse_inference(&duplicate).is_err());
    }

    #[test]
    fn calibration_test_partition_overlap_is_rejected() {
        let test_partition = CALIBRATION.replace("CALIBRATION", "TEST");
        assert!(parse_calibration(&test_partition).is_err());
    }

    #[test]
    fn row_permutation_preserves_calibration_and_inference_result() {
        let mut calibration_rows: Vec<&str> = CALIBRATION.lines().collect();
        let calibration_header = calibration_rows.remove(0);
        calibration_rows.reverse();
        let calibration_reordered =
            format!("{calibration_header}\n{}\n", calibration_rows.join("\n"));

        let mut inference_rows: Vec<&str> = INFERENCE.lines().collect();
        let inference_header = inference_rows.remove(0);
        inference_rows.reverse();
        let inference_reordered = format!("{inference_header}\n{}\n", inference_rows.join("\n"));

        let (_, _, original) = composed(CALIBRATION, INFERENCE);
        let (_, _, reordered) = composed(&calibration_reordered, &inference_reordered);
        assert_eq!(original, reordered);
    }
}

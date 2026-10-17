use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::process::ExitCode;

const SCHEMA_VERSION: &str = "0.1.0";
const BASIS_POINTS: i64 = 10_000;
const FALSE_ALARM_BUDGET_BPS: i64 = 100;
const TRAINING_EPISODES: usize = 128;
const HELD_OUT_NULL_EPISODES: usize = 256;
const EPISODE_LENGTH: usize = 128;
const SHIFTED_EPISODES_PER_CONDITION: usize = 64;
const SHIFT_ONSET: usize = 32;
const SHIFTED_EPISODE_LENGTH: usize = 96;
const MAX_DETECTION_DELAY_FOR_TARGET: usize = 5;
const RECALIBRATION_COST_UNITS: i64 = 7;
const LOSS_PER_EXPOSED_OBSERVATION_UNITS: i64 = 4;
const CUSUM_REFERENCE_K_MILLI: i64 = 500;
const EWMA_LAMBDA_BPS: i64 = 2_500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DetectorKind {
    Shewhart,
    Cusum,
    Ewma,
}

impl DetectorKind {
    fn id(self) -> &'static str {
        match self {
            Self::Shewhart => "shewhart_absolute_threshold",
            Self::Cusum => "two_sided_cusum",
            Self::Ewma => "two_sided_ewma",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct DetectorParameters {
    threshold_milli: i64,
}

#[derive(Clone, Copy, Debug)]
struct TunedDetector {
    kind: DetectorKind,
    parameters: DetectorParameters,
    training_false_alarm_episodes: usize,
    training_false_alarm_rate_bps: i64,
}

#[derive(Clone, Copy, Debug)]
struct ShiftCondition {
    condition_id: &'static str,
    magnitude_milli: i64,
    direction: i64,
    gradual: bool,
}

#[derive(Clone, Copy, Debug)]
struct ConditionMetrics {
    episodes: usize,
    detected_after_shift: usize,
    detected_within_target_delay: usize,
    missed: usize,
    pre_shift_false_alarm: usize,
    delay_samples: Vec<usize>,
    exposed_samples: usize,
}

#[derive(Clone, Debug)]
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 0x9e37_79b9_7f4a_7c15 } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.state = value;
        value.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    // Sum-of-uniforms approximation to standardized, centered noise.
    // This is a deterministic benchmark generator, not a physical noise model.
    fn residual_milli(&mut self) -> i64 {
        let total: i64 = (0..12).map(|_| (self.next_u64() % 10_000) as i64).sum();
        (total - 59_994) / 10
    }
}

fn generate_stream(
    seed: u64,
    length: usize,
    shift_start: Option<usize>,
    shift_magnitude_milli: i64,
    gradual: bool,
) -> Vec<i64> {
    let mut rng = Rng::new(seed);
    (0..length)
        .map(|index| {
            let noise = rng.residual_milli();
            let shift = match shift_start {
                Some(start) if index >= start && gradual => {
                    let steps = (index - start + 1).min(20) as i64;
                    shift_magnitude_milli * steps / 20
                }
                Some(start) if index >= start => shift_magnitude_milli,
                _ => 0,
            };
            noise + shift
        })
        .collect()
}

fn detect_first_alarm(
    series: &[i64],
    kind: DetectorKind,
    parameters: DetectorParameters,
) -> Option<usize> {
    let threshold = parameters.threshold_milli;
    let mut cusum_positive = 0_i64;
    let mut cusum_negative = 0_i64;
    let mut ewma_milli = 0_i64;
    for (index, observation) in series.iter().copied().enumerate() {
        let alarm = match kind {
            DetectorKind::Shewhart => observation.abs() >= threshold,
            DetectorKind::Cusum => {
                cusum_positive = 0.max(cusum_positive + observation - CUSUM_REFERENCE_K_MILLI);
                cusum_negative = 0.max(cusum_negative - observation - CUSUM_REFERENCE_K_MILLI);
                cusum_positive >= threshold || cusum_negative >= threshold
            }
            DetectorKind::Ewma => {
                ewma_milli = (EWMA_LAMBDA_BPS * observation
                    + (BASIS_POINTS - EWMA_LAMBDA_BPS) * ewma_milli)
                    / BASIS_POINTS;
                ewma_milli.abs() >= threshold
            }
        };
        if alarm {
            return Some(index);
        }
    }
    None
}

fn candidate_thresholds(kind: DetectorKind) -> Vec<i64> {
    match kind {
        DetectorKind::Shewhart => (2_000..=8_000).step_by(250).collect(),
        DetectorKind::Cusum => (2_500..=50_000).step_by(2_500).collect(),
        DetectorKind::Ewma => (500..=5_000).step_by(250).collect(),
    }
}

fn false_alarm_episodes(
    kind: DetectorKind,
    threshold_milli: i64,
    seeds: std::ops::Range<u64>,
    length: usize,
) -> usize {
    let parameters = DetectorParameters { threshold_milli };
    seeds.filter(|seed| {
        let stable = generate_stream(*seed, length, None, 0, false);
        detect_first_alarm(&stable, kind, parameters).is_some()
    }).count()
}

fn tune_detector(kind: DetectorKind) -> Result<TunedDetector, String> {
    // Threshold selection is permitted to inspect calibration-partition null
    // streams only. Held-out null and shifted streams are generated elsewhere.
    let candidate = candidate_thresholds(kind)
        .into_iter()
        .map(|threshold| {
            let alarms = false_alarm_episodes(
                kind,
                threshold,
                10_000..10_000 + TRAINING_EPISODES as u64,
                EPISODE_LENGTH,
            );
            (threshold, alarms)
        })
        .find(|(_, alarms)| {
            (*alarms as i64) * BASIS_POINTS
                <= FALSE_ALARM_BUDGET_BPS * TRAINING_EPISODES as i64
        });
    let Some((threshold_milli, alarms)) = candidate else {
        return Err(format!("{} has no threshold in the registered grid under the false-alarm budget", kind.id()));
    };
    Ok(TunedDetector {
        kind,
        parameters: DetectorParameters { threshold_milli },
        training_false_alarm_episodes: alarms,
        training_false_alarm_rate_bps: (alarms as i64) * BASIS_POINTS / TRAINING_EPISODES as i64,
    })
}

fn shift_conditions() -> Vec<ShiftCondition> {
    let mut values = Vec::new();
    for magnitude in [250_i64, 500, 1_000, 2_000] {
        for (direction_id, direction) in [("positive", 1_i64), ("negative", -1_i64)] {
            values.push(ShiftCondition {
                condition_id: match (magnitude, direction_id) {
                    (250, "positive") => "abrupt_pos_0_25_sigma",
                    (250, "negative") => "abrupt_neg_0_25_sigma",
                    (500, "positive") => "abrupt_pos_0_50_sigma",
                    (500, "negative") => "abrupt_neg_0_50_sigma",
                    (1_000, "positive") => "abrupt_pos_1_sigma",
                    (1_000, "negative") => "abrupt_neg_1_sigma",
                    (2_000, "positive") => "abrupt_pos_2_sigma",
                    _ => "abrupt_neg_2_sigma",
                },
                magnitude_milli: magnitude,
                direction,
                gradual: false,
            });
        }
    }
    values.push(ShiftCondition {
        condition_id: "gradual_pos_1_sigma_over_20_samples",
        magnitude_milli: 1_000,
        direction: 1,
        gradual: true,
    });
    values
}

fn evaluate_condition(detector: TunedDetector, condition_index: usize, condition: ShiftCondition) -> ConditionMetrics {
    let mut metrics = ConditionMetrics {
        episodes: SHIFTED_EPISODES_PER_CONDITION,
        detected_after_shift: 0,
        detected_within_target_delay: 0,
        missed: 0,
        pre_shift_false_alarm: 0,
        delay_samples: Vec::new(),
        exposed_samples: 0,
    };
    for episode in 0..SHIFTED_EPISODES_PER_CONDITION {
        let seed = 500_000_u64 + condition_index as u64 * 10_000 + episode as u64;
        let series = generate_stream(
            seed,
            SHIFTED_EPISODE_LENGTH,
            Some(SHIFT_ONSET),
            condition.magnitude_milli * condition.direction,
            condition.gradual,
        );
        match detect_first_alarm(&series, detector.kind, detector.parameters) {
            Some(index) if index < SHIFT_ONSET => {
                metrics.pre_shift_false_alarm += 1;
            }
            Some(index) => {
                let delay = index - SHIFT_ONSET + 1;
                metrics.detected_after_shift += 1;
                metrics.delay_samples.push(delay);
                if delay <= MAX_DETECTION_DELAY_FOR_TARGET {
                    metrics.detected_within_target_delay += 1;
                }
                metrics.exposed_samples += delay.saturating_sub(1);
            }
            None => {
                metrics.missed += 1;
                metrics.exposed_samples += SHIFTED_EPISODE_LENGTH - SHIFT_ONSET;
            }
        }
    }
    metrics
}

fn percentile(sorted_values: &[usize], percentile_bps: usize) -> Option<usize> {
    if sorted_values.is_empty() {
        return None;
    }
    let rank_one_based = (sorted_values.len() * percentile_bps).div_ceil(10_000);
    sorted_values.get(rank_one_based.saturating_sub(1)).copied()
}

fn condition_json(
    detector: TunedDetector,
    condition: ShiftCondition,
    metrics: &ConditionMetrics,
) -> Value {
    let mut delays = metrics.delay_samples.clone();
    delays.sort_unstable();
    json!({
        "condition_id": condition.condition_id,
        "shift_type": if condition.gradual { "gradual" } else { "abrupt" },
        "shift_magnitude_sigma_milli": condition.magnitude_milli,
        "shift_direction": if condition.direction > 0 { "positive" } else { "negative" },
        "episodes": metrics.episodes,
        "detected_after_shift": metrics.detected_after_shift,
        "detection_probability_bps": (metrics.detected_after_shift as i64) * BASIS_POINTS / metrics.episodes as i64,
        "detected_within_5_samples": metrics.detected_within_target_delay,
        "detection_within_5_samples_probability_bps": (metrics.detected_within_target_delay as i64) * BASIS_POINTS / metrics.episodes as i64,
        "missed": metrics.missed,
        "miss_rate_bps": (metrics.missed as i64) * BASIS_POINTS / metrics.episodes as i64,
        "pre_shift_false_alarm_count": metrics.pre_shift_false_alarm,
        "detection_delay_samples": {
            "median": percentile(&delays, 5_000),
            "p90": percentile(&delays, 9_000),
            "p95": percentile(&delays, 9_500)
        },
        "detected_episode_count_for_delay_quantiles": delays.len(),
        "total_post_shift_observations_exposed_before_detection": metrics.exposed_samples,
        "mean_exposed_observations_per_episode_milli": metrics.exposed_samples as i64 * 1_000 / metrics.episodes as i64,
        "synthetic_downstream_loss_units": metrics.exposed_samples as i64 * LOSS_PER_EXPOSED_OBSERVATION_UNITS,
        "synthetic_recalibration_cost_for_pre_shift_false_alarms": metrics.pre_shift_false_alarm as i64 * RECALIBRATION_COST_UNITS,
        "uses_held_out_streams": true,
        "detector": detector.kind.id()
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn execute(source_commit: &str) -> Result<Value, String> {
    if source_commit.len() != 40
        || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be a 40-character lowercase Git SHA-1".to_owned());
    }

    let kinds = [DetectorKind::Shewhart, DetectorKind::Cusum, DetectorKind::Ewma];
    let mut tuned = Vec::new();
    for kind in kinds {
        let detector = tune_detector(kind)?;
        if detector.training_false_alarm_rate_bps > FALSE_ALARM_BUDGET_BPS {
            return Err(format!("{} calibration false-alarm budget exceeded", kind.id()));
        }
        tuned.push(detector);
    }

    let heldout_null = tuned.iter().map(|detector| {
        let alarms = false_alarm_episodes(
            detector.kind,
            detector.parameters.threshold_milli,
            200_000..200_000 + HELD_OUT_NULL_EPISODES as u64,
            EPISODE_LENGTH,
        );
        json!({
            "detector": detector.kind.id(),
            "episodes": HELD_OUT_NULL_EPISODES,
            "false_alarm_episodes": alarms,
            "false_alarm_rate_bps": alarms as i64 * BASIS_POINTS / HELD_OUT_NULL_EPISODES as i64,
            "target_false_alarm_rate_bps": FALSE_ALARM_BUDGET_BPS,
            "held_out_from_threshold_selection": true
        })
    }).collect::<Vec<_>>();

    let conditions = shift_conditions();
    let mut detector_reports = Vec::new();
    for detector in tuned {
        let condition_reports = conditions.iter().copied().enumerate().map(|(index, condition)| {
            let metrics = evaluate_condition(detector, index, condition);
            condition_json(detector, condition, &metrics)
        }).collect::<Vec<_>>();
        let total_shift_episodes = conditions.len() * SHIFTED_EPISODES_PER_CONDITION;
        let total_prealarm: i64 = condition_reports.iter().map(|report| {
            report["pre_shift_false_alarm_count"].as_i64().unwrap_or(0)
        }).sum();
        let total_exposed: i64 = condition_reports.iter().map(|report| {
            report["total_post_shift_observations_exposed_before_detection"].as_i64().unwrap_or(0)
        }).sum();
        detector_reports.push(json!({
            "detector_id": detector.kind.id(),
            "frozen_threshold_milli": detector.parameters.threshold_milli,
            "cusum_reference_k_milli": if detector.kind == DetectorKind::Cusum { Some(CUSUM_REFERENCE_K_MILLI) } else { None },
            "ewma_lambda_bps": if detector.kind == DetectorKind::Ewma { Some(EWMA_LAMBDA_BPS) } else { None },
            "threshold_selection": {
                "training_null_seed_range": [10_000, 10_000 + TRAINING_EPISODES as u64],
                "episodes": TRAINING_EPISODES,
                "length": EPISODE_LENGTH,
                "false_alarm_episodes": detector.training_false_alarm_episodes,
                "false_alarm_rate_bps": detector.training_false_alarm_rate_bps,
                "maximum_rate_bps": FALSE_ALARM_BUDGET_BPS,
                "threshold_frozen_before_held_out_scoring": true
            },
            "shift_condition_count": conditions.len(),
            "total_shift_episodes": total_shift_episodes,
            "pre_shift_false_alarm_count_over_shifted_streams": total_prealarm,
            "total_post_shift_observations_exposed_before_detection": total_exposed,
            "synthetic_downstream_loss_units": total_exposed * LOSS_PER_EXPOSED_OBSERVATION_UNITS,
            "conditions": condition_reports
        }));
    }

    let model_spec = json!({
        "schema_version": SCHEMA_VERSION,
        "fixture_type": "synthetic_seeded_drift_detector_comparison",
        "noise_generator": "xorshift64star_sum_of_12_uniforms_centered_and_scaled_to_milli_sigma",
        "training_null_seed_range": [10_000, 10_000 + TRAINING_EPISODES as u64],
        "held_out_null_seed_range": [200_000, 200_000 + HELD_OUT_NULL_EPISODES as u64],
        "held_out_shift_seed_start": 500_000,
        "training_episodes": TRAINING_EPISODES,
        "held_out_null_episodes": HELD_OUT_NULL_EPISODES,
        "episode_length": EPISODE_LENGTH,
        "shifted_episode_length": SHIFTED_EPISODE_LENGTH,
        "shift_onset_zero_based": SHIFT_ONSET,
        "shift_episodes_per_condition": SHIFTED_EPISODES_PER_CONDITION,
        "shift_magnitudes_sigma_milli": [250, 500, 1000, 2000],
        "shift_directions": ["positive", "negative"],
        "gradual_shift": "positive 1 sigma ramp over 20 observations",
        "false_alarm_budget_bps": FALSE_ALARM_BUDGET_BPS,
        "cusum_reference_k_milli": CUSUM_REFERENCE_K_MILLI,
        "ewma_lambda_bps": EWMA_LAMBDA_BPS,
        "detectors": [DetectorKind::Shewhart.id(), DetectorKind::Cusum.id(), DetectorKind::Ewma.id()],
        "recalibration_cost_units": RECALIBRATION_COST_UNITS,
        "loss_per_exposed_observation_units": LOSS_PER_EXPOSED_OBSERVATION_UNITS
    });
    let model_bytes = serde_json::to_vec(&model_spec).map_err(|error| error.to_string())?;
    let source_sha256 = sha256_hex(include_str!("drift_detector_comparison_reference.rs").as_bytes());
    let calibration_budget_pass = detector_reports.iter().all(|detector| {
        detector["threshold_selection"]["false_alarm_rate_bps"].as_i64().unwrap_or(i64::MAX)
            <= FALSE_ALARM_BUDGET_BPS
    });
    if detector_reports.len() != 3
        || conditions.len() != 9
        || !calibration_budget_pass
        || heldout_null.len() != 3
        || detector_reports.iter().any(|detector| detector["conditions"].as_array().map_or(true, |items| items.len() != 9))
    {
        return Err("drift detector comparison receipt invariant failed".to_owned());
    }

    Ok(json!({
        "schema_version": SCHEMA_VERSION,
        "result": "pass",
        "source_commit": source_commit,
        "reference_source_sha256": source_sha256,
        "model_specification_sha256": sha256_hex(&model_bytes),
        "fixture_type": "synthetic_seeded_drift_detector_comparison",
        "evidence_boundary": "Contract and deterministic synthetic comparison only; no field detector performance is established.",
        "summary": {
            "detector_count": detector_reports.len(),
            "shift_condition_count": conditions.len(),
            "shift_episodes_per_condition": SHIFTED_EPISODES_PER_CONDITION,
            "total_shift_episodes_per_detector": conditions.len() * SHIFTED_EPISODES_PER_CONDITION,
            "held_out_null_episodes": HELD_OUT_NULL_EPISODES,
            "training_false_alarm_budget_bps": FALSE_ALARM_BUDGET_BPS,
            "all_training_thresholds_within_false_alarm_budget": calibration_budget_pass,
            "thresholds_fitted_on_null_training_data_only": true,
            "held_out_streams_used_for_final_metrics": true,
            "detectors_are_not_ranked_by_delay_alone": true
        },
        "held_out_null_results": heldout_null,
        "detector_comparisons": detector_reports,
        "acceptance_targets": {
            "training_false_alarm_rate_bps_at_most": FALSE_ALARM_BUDGET_BPS,
            "report_held_out_false_alarm_rate": true,
            "report_detection_probability_and_delay_by_shift": true,
            "report_pre_shift_false_alarms_and_pre_alarm_exposure": true,
            "report_downstream_exposure_and_recalibration_cost": true,
            "keep_metadata_envelope_gate_separate": true,
            "require_independent_held_out_physical_streams_before_real_world_claims": true
        },
        "partition_integrity": {
            "training_null_seed_range": [10_000, 10_000 + TRAINING_EPISODES as u64],
            "held_out_null_seed_range": [200_000, 200_000 + HELD_OUT_NULL_EPISODES as u64],
            "held_out_shift_seed_start": 500_000,
            "seed_ranges_disjoint": true,
            "held_out_streams_used_for_threshold_tuning": false,
            "test_data_used_for_fit_or_calibration": false
        },
        "model_specification": model_spec
    }))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut source_commit = None;
    let mut index = 1;
    while index < args.len() {
        if args[index] == "--source-commit" && index + 1 < args.len() {
            source_commit = Some(args[index + 1].as_str());
            index += 2;
        } else {
            eprintln!("unknown or incomplete argument: {}", args[index]);
            return ExitCode::from(2);
        }
    }
    let Some(source_commit) = source_commit else {
        eprintln!("missing required --source-commit <40-character-lowercase-git-sha1>");
        return ExitCode::from(2);
    };
    match execute(source_commit) {
        Ok(receipt) => {
            println!("{}", serde_json::to_string_pretty(&receipt).expect("JSON serialization should succeed"));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("drift detector comparison failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_is_deterministic_and_seed_sensitive() {
        assert_eq!(generate_stream(7, 50, None, 0, false), generate_stream(7, 50, None, 0, false));
        assert_ne!(generate_stream(7, 50, None, 0, false), generate_stream(8, 50, None, 0, false));
    }

    #[test]
    fn calibration_threshold_selection_uses_only_training_null_streams() {
        for kind in [DetectorKind::Shewhart, DetectorKind::Cusum, DetectorKind::Ewma] {
            let tuned = tune_detector(kind).unwrap();
            assert!(tuned.training_false_alarm_rate_bps <= FALSE_ALARM_BUDGET_BPS);
            assert!(candidate_thresholds(kind).contains(&tuned.parameters.threshold_milli));
        }
    }

    #[test]
    fn all_detector_implementations_are_causal_on_prefixes() {
        let series = generate_stream(90, 80, Some(30), 1_000, false);
        for kind in [DetectorKind::Shewhart, DetectorKind::Cusum, DetectorKind::Ewma] {
            let parameters = DetectorParameters { threshold_milli: 2_000 };
            if let Some(index) = detect_first_alarm(&series, kind, parameters) {
                assert_eq!(detect_first_alarm(&series[..=index], kind, parameters), Some(index));
            }
        }
    }

    #[test]
    fn abrupt_shift_fixtures_cover_both_directions_and_multiple_magnitudes() {
        let conditions = shift_conditions();
        assert_eq!(conditions.len(), 9);
        assert!(conditions.iter().any(|condition| condition.condition_id == "abrupt_pos_0_25_sigma"));
        assert!(conditions.iter().any(|condition| condition.condition_id == "abrupt_neg_2_sigma"));
        assert!(conditions.iter().any(|condition| condition.gradual));
    }

    #[test]
    fn shift_is_not_visible_before_its_declared_onset() {
        let stable = generate_stream(123, SHIFT_ONSET, Some(SHIFT_ONSET), 2_000, false);
        let changed = generate_stream(123, SHIFTED_EPISODE_LENGTH, Some(SHIFT_ONSET), 2_000, false);
        assert_eq!(stable, changed[..SHIFT_ONSET]);
    }

    #[test]
    fn percentile_reports_are_monotone_for_sorted_values() {
        let values = [1_usize, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile(&values, 5_000), Some(5));
        assert_eq!(percentile(&values, 9_000), Some(9));
        assert_eq!(percentile(&values, 9_500), Some(10));
        assert_eq!(percentile(&[], 9_000), None);
    }

    #[test]
    fn receipt_binds_threshold_model_and_held_out_metrics() {
        let receipt = execute(&"a".repeat(40)).unwrap();
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["summary"]["detector_count"], 3);
        assert_eq!(receipt["summary"]["shift_condition_count"], 9);
        assert_eq!(receipt["summary"]["total_shift_episodes_per_detector"], 576);
        assert_eq!(receipt["partition_integrity"]["seed_ranges_disjoint"], true);
        assert_eq!(receipt["partition_integrity"]["held_out_streams_used_for_threshold_tuning"], false);
        assert_eq!(receipt["summary"]["all_training_thresholds_within_false_alarm_budget"], true);
        assert_eq!(receipt["detector_comparisons"][0]["frozen_threshold_milli"], 4_000);
        assert_eq!(receipt["detector_comparisons"][1]["frozen_threshold_milli"], 10_000);
        assert_eq!(receipt["detector_comparisons"][2]["frozen_threshold_milli"], 1_500);
        assert_eq!(receipt["held_out_null_results"][0]["false_alarm_episodes"], 0);
        assert_eq!(receipt["held_out_null_results"][1]["false_alarm_episodes"], 0);
        assert_eq!(receipt["held_out_null_results"][2]["false_alarm_episodes"], 4);
        assert_eq!(receipt["detector_comparisons"][0]["conditions"][4]["detected_after_shift"], 4);
        assert_eq!(receipt["detector_comparisons"][0]["conditions"][4]["missed"], 60);
        assert_eq!(receipt["detector_comparisons"][1]["conditions"][4]["detected_after_shift"], 64);
        assert_eq!(receipt["detector_comparisons"][1]["conditions"][8]["detected_after_shift"], 64);
        assert_eq!(receipt["detector_comparisons"][2]["conditions"][4]["detected_after_shift"], 60);
        assert_eq!(receipt["detector_comparisons"][2]["conditions"][6]["detected_within_5_samples"], 39);
        assert_eq!(receipt["model_specification_sha256"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn missing_commit_or_unregistered_detector_configuration_fails_closed() {
        assert!(execute("bad-commit").is_err());
        let thresholds = candidate_thresholds(DetectorKind::Shewhart);
        assert!(!thresholds.contains(&-1));
    }
}

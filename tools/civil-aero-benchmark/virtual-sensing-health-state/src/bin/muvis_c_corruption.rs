// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Tier-0 MuViS-C protocol adapter: deterministic, synthetic reference only.
// This is not a reproduction of MuViS-C's datasets, trained models or results.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const FIXTURE: &[u8] = include_bytes!("../../data/muvis-c-corruption-v0.1.json");
const CLAIM_CEILING: &str = "synthetic_single_channel_corruption_virtual_sensing_protocol_only";
const FAILURE_MODES: [&str; 10] = [
    "bias",
    "noise",
    "scaling",
    "time_varying_scaling",
    "linear_drift",
    "nonlinear_drift",
    "outliers",
    "trimming_variable",
    "trimming_constant",
    "hard_fault",
];

#[derive(Clone, Debug, Deserialize)]
struct Dataset {
    dataset_id: String,
    domain: String,
    sample_count: usize,
    time_steps: usize,
    channel_ids: Vec<String>,
    input_layout: String,
    normalization: String,
    source_status: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Predictor {
    model_id: String,
    model_class: String,
    channel_weights: Vec<f64>,
    training_target_mean: f64,
    target_units: String,
}

#[derive(Clone, Debug, Deserialize)]
struct MetricDefinitions {
    #[serde(rename = "mPC")]
    mpc: String,
    #[serde(rename = "rPC")]
    rpc: String,
    s_cross: String,
    #[serde(rename = "nPC")]
    npc: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Protocol {
    k: f64,
    p_max: f64,
    alpha: f64,
    severity_grid: Vec<f64>,
    single_channel_corruption: bool,
    uncorrupted_channels_remain_clean: bool,
    bootstrap_replicates: usize,
    bootstrap_index_seed: u64,
    bootstrap_index_method: String,
    noise_draw_seed: u64,
    noise_draw_method: String,
    outlier_uniform_seed: u64,
    outlier_delta_seed: u64,
    uniform_draw_method: String,
    metric_definitions: MetricDefinitions,
}

#[derive(Clone, Debug, Deserialize)]
struct NormalizationStatistics {
    method: String,
    mean: Vec<f64>,
    std: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct ExpectedWorstCase {
    failure_mode: String,
    channel_id: String,
    severity: f64,
    rmse: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct ExpectedMetrics {
    clean_rmse: f64,
    naive_rmse: f64,
    mpc: f64,
    rpc: f64,
    npc: f64,
    s_cross: Option<f64>,
    grid_result_count: usize,
    worst_case: ExpectedWorstCase,
}

#[derive(Clone, Debug, Deserialize)]
struct Fixture {
    schema_version: String,
    problem_id: String,
    claim_ceiling: String,
    dataset: Dataset,
    predictor: Predictor,
    protocol: Protocol,
    windows: Vec<Vec<Vec<f64>>>,
    target: Vec<f64>,
    target_noise: Vec<f64>,
    noise_standard_normal: Vec<Vec<Vec<f64>>>,
    outlier_uniform: Vec<Vec<Vec<f64>>>,
    outlier_delta_unit: Vec<Vec<Vec<f64>>>,
    bootstrap_resample_indices: Vec<Vec<usize>>,
    raw_windows: Vec<Vec<Vec<f64>>>,
    normalization_statistics: NormalizationStatistics,
    expected_metrics: ExpectedMetrics,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct GridResult {
    failure_mode: String,
    channel_id: String,
    severity: f64,
    rmse: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct WorstCase {
    failure_mode: String,
    channel_id: String,
    severity: f64,
    rmse: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    problem_id: String,
    source_sha256: String,
    source_bytes: usize,
    claim_ceiling: String,
    sample_count: usize,
    time_steps: usize,
    channel_count: usize,
    failure_mode_count: usize,
    severity_count: usize,
    bootstrap_replicates: usize,
    clean_rmse: f64,
    naive_rmse: f64,
    mpc: f64,
    rpc: f64,
    npc: f64,
    s_cross: Option<f64>,
    worst_case: WorstCase,
    grid_result_count: usize,
    grid: Vec<GridResult>,
}

fn fail<T>(message: impl Into<String>) -> Result<T, String> {
    Err(message.into())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn lcg_next(state: &mut u32) -> f64 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *state as f64 / 4_294_967_296.0
}

fn round_12(value: f64) -> f64 {
    (value * 1.0e12).round() / 1.0e12
}

fn validate_cube(
    cube: &[Vec<Vec<f64>>],
    samples: usize,
    time_steps: usize,
    channels: usize,
    label: &str,
) -> Result<(), String> {
    if cube.len() != samples
        || cube.iter().any(|sample| {
            sample.len() != time_steps || sample.iter().any(|step| step.len() != channels)
        })
    {
        return fail(format!("{label}: expected N×T×C cube dimensions"));
    }
    if cube
        .iter()
        .flatten()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return fail(format!("{label}: non-finite sample"));
    }
    Ok(())
}

fn validate(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != "aero-muvis-c-protocol-reference/0.1.0" {
        return fail("unsupported MuViS-C adapter schema");
    }
    if fixture.claim_ceiling != CLAIM_CEILING {
        return fail("MuViS-C adapter claim ceiling changed or weakened");
    }
    if fixture.problem_id.trim().is_empty() || fixture.dataset.dataset_id.trim().is_empty() {
        return fail("problem and dataset IDs must be non-empty");
    }
    let n = fixture.dataset.sample_count;
    let t = fixture.dataset.time_steps;
    let c = fixture.dataset.channel_ids.len();
    if n < 2 || t < 2 || c != 3 {
        return fail("v0.1 requires N>=2, T>=2 and exactly three channels");
    }
    if fixture.dataset.input_layout != "N_T_C"
        || fixture.dataset.normalization != "per_channel_zscore_frozen_fixture_statistics"
        || fixture.dataset.source_status != "synthetic_known_answer_only"
        || fixture.dataset.domain != "synthetic_aerospace_sensor_windows"
    {
        return fail("dataset identity or preprocessing assumptions are outside v0.1");
    }
    let mut ids = BTreeSet::new();
    for id in &fixture.dataset.channel_ids {
        if id.trim().is_empty() || !ids.insert(id.as_str()) {
            return fail("channel IDs must be non-empty and unique");
        }
    }
    if fixture.predictor.model_id != "frozen_linear_virtual_sensor_v0"
        || fixture.predictor.model_class != "linear_temporal_mean_regressor"
        || fixture.predictor.channel_weights.len() != c
        || fixture
            .predictor
            .channel_weights
            .iter()
            .any(|value| !value.is_finite())
        || !fixture.predictor.training_target_mean.is_finite()
        || fixture.predictor.target_units.trim().is_empty()
    {
        return fail("frozen predictor contract is invalid");
    }
    let protocol = &fixture.protocol;
    if protocol.k != 3.0
        || protocol.p_max != 0.3
        || protocol.alpha != 0.4
        || !protocol.single_channel_corruption
        || !protocol.uncorrupted_channels_remain_clean
        || protocol.bootstrap_replicates != 200
        || protocol.bootstrap_index_seed != 260916
        || protocol.bootstrap_index_method != "LCG32_1664525_1013904223_mod_sample_count"
        || protocol.noise_draw_seed != 260917
        || protocol.noise_draw_method != "LCG32_BOX_MULLER_PRECOMPUTED"
        || protocol.outlier_uniform_seed != 260918
        || protocol.outlier_delta_seed != 260919
        || protocol.uniform_draw_method != "LCG32_PRECOMPUTED_UNIFORM_32"
    {
        return fail("MuViS-C protocol constants or atomic corruption rule changed");
    }
    if protocol.severity_grid.len() != 20 {
        return fail("v0.1 requires exactly 20 severity levels");
    }
    for (index, severity) in protocol.severity_grid.iter().enumerate() {
        let expected = (index as f64 + 1.0) * 0.05;
        if !severity.is_finite() || (*severity - expected).abs() > 1.0e-12 {
            return fail("severity grid must be 0.05, 0.10, ..., 1.00");
        }
    }
    if fixture.target.len() != n || fixture.target.iter().any(|value| !value.is_finite()) {
        return fail("target vector dimension/value invalid");
    }
    if fixture.target_noise.len() != n
        || fixture.target_noise.iter().any(|value| !value.is_finite())
    {
        return fail("target_noise vector dimension/value invalid");
    }
    validate_cube(&fixture.windows, n, t, c, "windows")?;
    validate_cube(&fixture.raw_windows, n, t, c, "raw_windows")?;
    let normalization = &fixture.normalization_statistics;
    if normalization.method != "population_zscore_over_frozen_synthetic_window_corpus"
        || normalization.mean.len() != c
        || normalization.std.len() != c
        || normalization.mean.iter().any(|value| !value.is_finite())
        || normalization
            .std
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return fail("frozen per-channel z-score statistics are invalid");
    }
    for channel in 0..c {
        let values: Vec<f64> = fixture
            .raw_windows
            .iter()
            .flat_map(|sample| sample.iter().map(move |step| step[channel]))
            .collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / values.len() as f64;
        let std = variance.sqrt();
        if (normalization.mean[channel] - mean).abs() > 1.0e-10
            || (normalization.std[channel] - std).abs() > 1.0e-10
        {
            return fail("z-score statistics do not match raw synthetic channel values");
        }
    }
    for sample in 0..n {
        for time in 0..t {
            for channel in 0..c {
                let normalized = (fixture.raw_windows[sample][time][channel]
                    - normalization.mean[channel])
                    / normalization.std[channel];
                if (normalized - fixture.windows[sample][time][channel]).abs() > 1.0e-10 {
                    return fail(
                        "standardized window differs from raw value and frozen channel statistics",
                    );
                }
            }
        }
    }
    validate_cube(
        &fixture.noise_standard_normal,
        n,
        t,
        c,
        "noise_standard_normal",
    )?;
    validate_cube(&fixture.outlier_uniform, n, t, c, "outlier_uniform")?;
    validate_cube(&fixture.outlier_delta_unit, n, t, c, "outlier_delta_unit")?;
    if fixture
        .outlier_uniform
        .iter()
        .flatten()
        .flatten()
        .any(|value| !(0.0..=1.0).contains(value))
    {
        return fail("outlier uniform draws must lie in [0,1]");
    }
    if fixture
        .outlier_delta_unit
        .iter()
        .flatten()
        .flatten()
        .any(|value| !(-1.0..=1.0).contains(value))
    {
        return fail("outlier delta-unit draws must lie in [-1,1]");
    }
    if fixture.bootstrap_resample_indices.len() != protocol.bootstrap_replicates
        || fixture
            .bootstrap_resample_indices
            .iter()
            .any(|row| row.len() != n || row.iter().any(|index| *index >= n))
    {
        return fail("bootstrap index matrix must be B×N with indices in 0..N");
    }
    let mut bootstrap_state = protocol.bootstrap_index_seed as u32;
    for row in 0..protocol.bootstrap_replicates {
        for column in 0..n {
            let expected = (lcg_next(&mut bootstrap_state) * 4_294_967_296.0) as usize % n;
            if fixture.bootstrap_resample_indices[row][column] != expected {
                return fail("bootstrap resample indices do not match declared seed/method");
            }
        }
    }

    let mut noise_state = protocol.noise_draw_seed as u32;
    let mut spare_normal: Option<f64> = None;
    for sample in 0..n {
        for time in 0..t {
            for channel in 0..c {
                let expected = if let Some(value) = spare_normal.take() {
                    value
                } else {
                    let u1 = lcg_next(&mut noise_state).max(1.0e-12);
                    let u2 = lcg_next(&mut noise_state);
                    let radius = (-2.0 * u1.ln()).sqrt();
                    let angle = 2.0 * std::f64::consts::PI * u2;
                    spare_normal = Some(round_12(radius * angle.sin()));
                    radius * angle.cos()
                };
                let expected = round_12(expected);
                if (fixture.noise_standard_normal[sample][time][channel] - expected).abs() > 1.0e-10
                {
                    return fail("normal noise draw does not match declared seed/method");
                }
            }
        }
    }

    let mut outlier_uniform_state = protocol.outlier_uniform_seed as u32;
    let mut outlier_delta_state = protocol.outlier_delta_seed as u32;
    for sample in 0..n {
        for time in 0..t {
            for channel in 0..c {
                let expected_uniform = round_12(lcg_next(&mut outlier_uniform_state));
                let expected_delta = round_12(2.0 * lcg_next(&mut outlier_delta_state) - 1.0);
                if (fixture.outlier_uniform[sample][time][channel] - expected_uniform).abs()
                    > 1.0e-12
                    || (fixture.outlier_delta_unit[sample][time][channel] - expected_delta).abs()
                        > 1.0e-12
                {
                    return fail("outlier draws do not match declared uniform seeds/method");
                }
            }
        }
    }
    if protocol.metric_definitions.mpc
        != "mean bootstrapped RMSE over failure_mode x channel x severity"
        || protocol.metric_definitions.rpc != "mPC / clean_RMSE; 1 means no average degradation"
        || protocol.metric_definitions.s_cross
            != "minimum severity where any failure/channel bootstrapped RMSE >= naive mean predictor RMSE; null if no crossing"
        || protocol.metric_definitions.npc != "mPC / naive_RMSE"
    {
        return fail("MuViS-C metric definitions were changed");
    }

    // Freeze target provenance rather than treating the scalar reference as opaque.
    for sample in 0..n {
        let means: Vec<f64> = (0..c)
            .map(|channel| {
                fixture.windows[sample]
                    .iter()
                    .map(|step| step[channel])
                    .sum::<f64>()
                    / t as f64
            })
            .collect();
        let expected_target =
            0.50 * means[0] - 0.27 * means[1] + 0.16 * means[2] + fixture.target_noise[sample];
        if (fixture.target[sample] - expected_target).abs() > 1.0e-10 {
            return fail("target vector does not match the frozen synthetic target equation");
        }
    }
    Ok(())
}

fn apply_failure(
    mode: &str,
    severity: f64,
    value: f64,
    noise_draw: f64,
    outlier_uniform: f64,
    outlier_delta_unit: f64,
    time_index: usize,
    time_steps: usize,
    protocol: &Protocol,
) -> Result<f64, String> {
    let s = severity;
    let k = protocol.k;
    let fraction = time_index as f64 / (time_steps - 1) as f64;
    let result = match mode {
        "bias" => value + s * k,
        "noise" => value + noise_draw * s * k,
        "scaling" => value * (1.0 + s * (k - 1.0)),
        "time_varying_scaling" => value * (1.0 + s * (k - 1.0) * fraction),
        "linear_drift" => value + s * k * fraction,
        "nonlinear_drift" => value + s * k * fraction.powi(2),
        "outliers" => {
            if outlier_uniform < s * protocol.p_max {
                value + outlier_delta_unit * k
            } else {
                value
            }
        }
        "trimming_variable" => {
            let h = (1.0 - s) * k;
            if value < -h {
                -h + protocol.alpha * (value + h)
            } else if value > h {
                h + protocol.alpha * (value - h)
            } else {
                value
            }
        }
        "trimming_constant" => {
            let h = (1.0 - s) * k;
            value.clamp(-h, h)
        }
        "hard_fault" => s * k,
        _ => return fail(format!("unknown failure mode {mode}")),
    };
    if result.is_finite() {
        Ok(result)
    } else {
        fail("corruption produced a non-finite value")
    }
}

fn predict(windows: &[Vec<Vec<f64>>], weights: &[f64]) -> Vec<f64> {
    windows
        .iter()
        .map(|sample| {
            let time_steps = sample.len() as f64;
            let mut prediction = 0.0;
            for (channel, weight) in weights.iter().enumerate() {
                let channel_mean =
                    sample.iter().map(|step| step[channel]).sum::<f64>() / time_steps;
                prediction += weight * channel_mean;
            }
            prediction
        })
        .collect()
}

fn bootstrap_mean_rmse(predictions: &[f64], target: &[f64], resamples: &[Vec<usize>]) -> f64 {
    let sum: f64 = resamples
        .iter()
        .map(|indices| {
            let mse = indices
                .iter()
                .map(|index| (predictions[*index] - target[*index]).powi(2))
                .sum::<f64>()
                / indices.len() as f64;
            mse.sqrt()
        })
        .sum();
    sum / resamples.len() as f64
}

fn evaluate(fixture: &Fixture, source_bytes: &[u8]) -> Result<Report, String> {
    validate(fixture)?;
    let n = fixture.dataset.sample_count;
    let t = fixture.dataset.time_steps;
    let c = fixture.dataset.channel_ids.len();
    let clean_predictions = predict(&fixture.windows, &fixture.predictor.channel_weights);
    let naive_predictions = vec![fixture.predictor.training_target_mean; n];
    let clean_rmse = bootstrap_mean_rmse(
        &clean_predictions,
        &fixture.target,
        &fixture.bootstrap_resample_indices,
    );
    let naive_rmse = bootstrap_mean_rmse(
        &naive_predictions,
        &fixture.target,
        &fixture.bootstrap_resample_indices,
    );
    if clean_rmse <= f64::EPSILON || naive_rmse <= f64::EPSILON {
        return fail("clean and naïve RMSE must be positive for normalized corruption metrics");
    }

    let mut grid =
        Vec::with_capacity(FAILURE_MODES.len() * c * fixture.protocol.severity_grid.len());
    for mode in FAILURE_MODES {
        for channel in 0..c {
            for severity in &fixture.protocol.severity_grid {
                let mut corrupted = fixture.windows.clone();
                for sample in 0..n {
                    for time in 0..t {
                        let raw = fixture.windows[sample][time][channel];
                        corrupted[sample][time][channel] = apply_failure(
                            mode,
                            *severity,
                            raw,
                            fixture.noise_standard_normal[sample][time][channel],
                            fixture.outlier_uniform[sample][time][channel],
                            fixture.outlier_delta_unit[sample][time][channel],
                            time,
                            t,
                            &fixture.protocol,
                        )?;
                    }
                }
                let predictions = predict(&corrupted, &fixture.predictor.channel_weights);
                let rmse = bootstrap_mean_rmse(
                    &predictions,
                    &fixture.target,
                    &fixture.bootstrap_resample_indices,
                );
                grid.push(GridResult {
                    failure_mode: mode.to_owned(),
                    channel_id: fixture.dataset.channel_ids[channel].clone(),
                    severity: *severity,
                    rmse,
                });
            }
        }
    }

    let mpc = grid.iter().map(|entry| entry.rmse).sum::<f64>() / grid.len() as f64;
    let rpc = mpc / clean_rmse;
    let npc = mpc / naive_rmse;
    let s_cross = grid
        .iter()
        .filter(|entry| entry.rmse >= naive_rmse)
        .map(|entry| entry.severity)
        .reduce(f64::min);
    let worst = grid
        .iter()
        .max_by(|a, b| a.rmse.total_cmp(&b.rmse))
        .expect("the MuViS-C grid has at least one entry");
    let worst_case = WorstCase {
        failure_mode: worst.failure_mode.clone(),
        channel_id: worst.channel_id.clone(),
        severity: worst.severity,
        rmse: worst.rmse,
    };

    Ok(Report {
        schema_version: "aero-muvis-c-protocol-reference-result/0.1.0".to_owned(),
        problem_id: fixture.problem_id.clone(),
        source_sha256: sha256_hex(source_bytes),
        source_bytes: source_bytes.len(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        sample_count: n,
        time_steps: t,
        channel_count: c,
        failure_mode_count: FAILURE_MODES.len(),
        severity_count: fixture.protocol.severity_grid.len(),
        bootstrap_replicates: fixture.protocol.bootstrap_replicates,
        clean_rmse,
        naive_rmse,
        mpc,
        rpc,
        npc,
        s_cross,
        worst_case,
        grid_result_count: grid.len(),
        grid,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture: Fixture = serde_json::from_slice(FIXTURE)?;
    let report = evaluate(&fixture, FIXTURE).map_err(std::io::Error::other)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_slice(FIXTURE).expect("frozen MuViS-C protocol fixture parses")
    }

    #[test]
    fn exhaustive_muvis_c_grid_has_ten_modes_three_channels_twenty_severities() {
        let data = fixture();
        let result = evaluate(&data, FIXTURE).expect("fixture evaluates");
        assert_eq!(result.failure_mode_count, 10);
        assert_eq!(result.channel_count, 3);
        assert_eq!(result.severity_count, 20);
        assert_eq!(result.bootstrap_replicates, 200);
        assert_eq!(result.grid_result_count, 600);
        assert_eq!(result.grid.len(), 600);
    }

    #[test]
    fn deterministic_bootstrap_and_corruption_replay() {
        let data = fixture();
        let first = evaluate(&data, FIXTURE).unwrap();
        let second = evaluate(&data, FIXTURE).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn severity_zero_is_identity_except_the_mean_imputation_hard_fault() {
        let data = fixture();
        let x = 0.75;
        for mode in FAILURE_MODES {
            let actual =
                apply_failure(mode, 0.0, x, 0.25, 0.5, -0.5, 3, 8, &data.protocol).unwrap();
            let expected = if mode == "hard_fault" { 0.0 } else { x };
            assert!((actual - expected).abs() < 1.0e-12, "{mode}");
        }
    }

    #[test]
    fn one_channel_corruption_leaves_other_channels_bit_identical() {
        let data = fixture();
        let mut corrupted = data.windows.clone();
        let channel = 1usize;
        for sample in 0..data.dataset.sample_count {
            for time in 0..data.dataset.time_steps {
                corrupted[sample][time][channel] = apply_failure(
                    "bias",
                    0.5,
                    data.windows[sample][time][channel],
                    data.noise_standard_normal[sample][time][channel],
                    data.outlier_uniform[sample][time][channel],
                    data.outlier_delta_unit[sample][time][channel],
                    time,
                    data.dataset.time_steps,
                    &data.protocol,
                )
                .unwrap();
                for other in 0..data.dataset.channel_ids.len() {
                    if other != channel {
                        assert_eq!(
                            corrupted[sample][time][other],
                            data.windows[sample][time][other]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn normalized_metrics_and_worst_case_are_finite() {
        let result = evaluate(&fixture(), FIXTURE).unwrap();
        assert!(result.clean_rmse.is_finite() && result.clean_rmse > 0.0);
        assert!(result.naive_rmse.is_finite() && result.naive_rmse > 0.0);
        assert!(result.mpc.is_finite() && result.mpc > 0.0);
        assert!(result.rpc.is_finite() && result.rpc > 0.0);
        assert!(result.npc.is_finite() && result.npc > 0.0);
        assert!(result.worst_case.rmse >= result.mpc);
    }

    #[test]
    fn frozen_muvis_c_metrics_match_known_answers() {
        let data = fixture();
        let result = evaluate(&data, FIXTURE).unwrap();
        let expected = &data.expected_metrics;
        for (actual, wanted) in [
            (result.clean_rmse, expected.clean_rmse),
            (result.naive_rmse, expected.naive_rmse),
            (result.mpc, expected.mpc),
            (result.rpc, expected.rpc),
            (result.npc, expected.npc),
            (result.worst_case.rmse, expected.worst_case.rmse),
        ] {
            assert!((actual - wanted).abs() < 1.0e-9, "{actual} != {wanted}");
        }
        assert_eq!(result.s_cross, expected.s_cross);
        assert_eq!(result.grid_result_count, expected.grid_result_count);
        assert_eq!(
            result.worst_case.failure_mode,
            expected.worst_case.failure_mode
        );
        assert_eq!(result.worst_case.channel_id, expected.worst_case.channel_id);
        assert_eq!(result.worst_case.severity, expected.worst_case.severity);
    }

    #[test]
    fn malformed_bootstrap_and_failure_draws_reject() {
        let mut data = fixture();
        data.bootstrap_resample_indices[0][0] = data.dataset.sample_count;
        assert!(validate(&data).is_err());

        let mut data = fixture();
        data.outlier_uniform[0][0][0] = 2.0;
        assert!(validate(&data).is_err());

        let mut data = fixture();
        data.protocol.severity_grid[0] = 0.0;
        assert!(validate(&data).is_err());
    }
}

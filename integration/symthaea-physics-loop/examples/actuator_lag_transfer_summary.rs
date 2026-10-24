//! Matched, episode-level actuator-lag transfer summary.
//!
//! This aggregator requires one unchanged training dataset and identical episode ordering
//! across direct/10ms/20ms/50ms evaluation profiles. It bootstraps episodes within scenario
//! families; correlated frames are never treated as independent samples.

use std::collections::BTreeMap;
use std::fs;

use serde_json::{json, Map, Value};

const BOOTSTRAP_REPLICATES: usize = 10_000;
const CONFIDENCE_LEVEL: f64 = 0.95;
const BOOTSTRAP_SEED: u64 = 0x4143_5455_4154_4f52;
const REFERENCE_TAU_S: f64 = 0.020;
const EPSILON_RMSE: f64 = 1.0e-9;

const MODEL_KEYS: [(&str, &str); 5] = [
    ("raw_state_ridge", "raw_state_ridge_rmse_m_s2"),
    ("compact_mlp", "compact_mlp_rmse_m_s2"),
    ("explicit_history_mlp", "explicit_history_mlp_rmse_m_s2"),
    ("persistent_hdc_ltc", "hdc_ltc_reservoir_rmse_m_s2"),
    ("ltc_reset_each_step_hdc", "hdc_ltc_reset_each_step_rmse_m_s2"),
];

#[derive(Debug, Clone)]
struct Episode {
    seed_hex: String,
    scenario_id: String,
    scenario_family: String,
    rmse: [f64; 5],
}

#[derive(Debug, Clone)]
struct Profile {
    evaluation_tau_s: f64,
    training_configuration: Value,
    evaluation_configuration: Value,
    training_dataset_sha256: String,
    source_receipt: Value,
    model_summary: Value,
    state_memory_ablation: Value,
    episodes: Vec<Episode>,
}

fn required_string(value: &Value, label: &str) -> String {
    value
        .as_str()
        .unwrap_or_else(|| panic!("missing string receipt field: {label}"))
        .to_owned()
}

fn required_number(value: &Value, label: &str) -> f64 {
    let number = value
        .as_f64()
        .unwrap_or_else(|| panic!("missing numeric receipt field: {label}"));
    assert!(number.is_finite(), "non-finite receipt number: {label}");
    number
}

fn read_profile(env_name: &str, expected_tau_s: f64) -> Profile {
    let path = std::env::var(env_name)
        .unwrap_or_else(|error| panic!("{env_name} must point to a profile receipt: {error}"));
    let bytes = fs::read(&path)
        .unwrap_or_else(|error| panic!("cannot read profile receipt {path}: {error}"));
    let root: Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|error| panic!("{path} is invalid JSON: {error}"));

    assert_eq!(
        root["comparison_id"].as_str(),
        Some("symtropy-residual-model-paired-comparison-v0.1"),
        "unsupported profile receipt: {path}"
    );
    assert_eq!(
        root["claim_ceiling"].as_str(),
        Some("paired_episode_level_one_step_prediction_comparison_conditional_on_smoke_scenario_families"),
        "unsupported claim ceiling in profile: {path}"
    );

    let tau = root["plant_configuration"]["evaluation"]["actuator_lag_time_constant_s"]
        .as_f64()
        .unwrap_or(0.0);
    assert!(
        (tau - expected_tau_s).abs() < 1.0e-9,
        "{path} evaluation time constant {tau} does not match expected {expected_tau_s}"
    );
    assert_eq!(
        root["plant_configuration"]["training"]["claim_ceiling"].as_str(),
        Some("internal_actuator_lag_stress_configuration_not_calibrated_real_flight_model")
    );
    assert_eq!(
        root["plant_configuration"]["evaluation"]["claim_ceiling"].as_str(),
        Some("internal_actuator_lag_stress_configuration_not_calibrated_real_flight_model")
    );
    let digest = required_string(
        &root["source_receipt"]["training_dataset_sha256"],
        "source_receipt.training_dataset_sha256",
    );
    assert_eq!(digest.len(), 64, "training dataset digest must be SHA-256 hex");
    assert!(
        digest.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "training dataset digest contains non-hexadecimal characters"
    );

    let declared_episodes = root["design"]["episodes"]
        .as_u64()
        .unwrap_or_else(|| panic!("{path} lacks design.episodes")) as usize;
    let rows = root["per_episode"]
        .as_array()
        .unwrap_or_else(|| panic!("{path} lacks per_episode array"));
    assert_eq!(rows.len(), declared_episodes, "episode count mismatch in {path}");
    assert!(!rows.is_empty(), "profile has no episodes: {path}");

    let episodes = rows.iter().enumerate().map(|(index, row)| {
        let seed_hex = required_string(&row["episode_seed_hex"], &format!("{path}.per_episode[{index}].episode_seed_hex"));
        assert_eq!(seed_hex.len(), 16, "episode seed must be fixed-width u64 hex");
        assert!(seed_hex.bytes().all(|byte| byte.is_ascii_hexdigit()), "invalid episode seed: {seed_hex}");
        let scenario_id = required_string(&row["scenario_id"], &format!("{path}.per_episode[{index}].scenario_id"));
        let scenario_family = required_string(&row["scenario_family"], &format!("{path}.per_episode[{index}].scenario_family"));
        let rmse = std::array::from_fn(|model_index| {
            let key = MODEL_KEYS[model_index].1;
            let value = required_number(&row[key], &format!("{path}.per_episode[{index}].{key}"));
            assert!(value >= 0.0, "negative RMSE in {path}.per_episode[{index}].{key}");
            value
        });
        Episode { seed_hex, scenario_id, scenario_family, rmse }
    }).collect::<Vec<_>>();

    Profile {
        evaluation_tau_s: tau,
        training_configuration: root["plant_configuration"]["training"].clone(),
        evaluation_configuration: root["plant_configuration"]["evaluation"].clone(),
        training_dataset_sha256: digest,
        source_receipt: root["source_receipt"].clone(),
        model_summary: root["model_summary"].clone(),
        state_memory_ablation: root["state_memory_ablation"].clone(),
        episodes,
    }
}

/// SplitMix64 gives repeatable bootstrap draws without an additional dependency.
#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn index(&mut self, length: usize) -> usize {
        assert!(length > 0);
        (self.next_u64() % length as u64) as usize
    }
}

fn family_indices(episodes: &[Episode]) -> BTreeMap<&str, Vec<usize>> {
    let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, episode) in episodes.iter().enumerate() {
        groups.entry(episode.scenario_family.as_str()).or_default().push(index);
    }
    groups
}

fn mean(values: &[f64]) -> f64 {
    assert!(!values.is_empty(), "cannot average an empty episode vector");
    values.iter().sum::<f64>() / values.len() as f64
}

fn bootstrap_ci(values: &[f64], episodes: &[Episode], seed: u64) -> [f64; 2] {
    assert_eq!(values.len(), episodes.len());
    assert!(!values.is_empty());
    let groups = family_indices(episodes);
    assert!(!groups.is_empty());
    let mut rng = SplitMix64::new(seed);
    let mut samples = Vec::with_capacity(BOOTSTRAP_REPLICATES);

    for _ in 0..BOOTSTRAP_REPLICATES {
        let mut total = 0.0;
        let mut count = 0usize;
        // Keep the observed scenario-family sample sizes fixed in every bootstrap draw.
        for indices in groups.values() {
            for _ in 0..indices.len() {
                total += values[indices[rng.index(indices.len())]];
                count += 1;
            }
        }
        assert_eq!(count, values.len());
        samples.push(total / count as f64);
    }

    samples.sort_by(f64::total_cmp);
    let lower = ((BOOTSTRAP_REPLICATES as f64 * (1.0 - CONFIDENCE_LEVEL) / 2.0).floor() as usize)
        .min(BOOTSTRAP_REPLICATES - 1);
    let upper = ((BOOTSTRAP_REPLICATES as f64 * (1.0 + CONFIDENCE_LEVEL) / 2.0).ceil() as usize)
        .saturating_sub(1)
        .min(BOOTSTRAP_REPLICATES - 1);
    [samples[lower], samples[upper]]
}

fn summarize(values: &[f64], episodes: &[Episode], seed: u64) -> Value {
    let average = mean(values);
    let ci = bootstrap_ci(values, episodes, seed);
    json!({
        "episode_count": values.len(),
        "mean_paired_rmse_delta_m_s2": average,
        "stratified_episode_bootstrap_95_ci_m_s2": ci,
        "bootstrap_replicates": BOOTSTRAP_REPLICATES,
        "bootstrap_unit": "episode",
        "stratification": "resample with replacement within each observed scenario family",
        "interpretation": "positive means more residual-prediction error than the matched 20ms evaluation profile; negative means less error"
    })
}

fn model_mean(summary: &Value, key: &str) -> f64 {
    required_number(&summary[key], &format!("model_summary.{key}"))
}

fn main() {
    let profiles = vec![
        read_profile("STFR_ACTUATOR_DIRECT_COMPARISON_PATH", 0.0),
        read_profile("STFR_ACTUATOR_10MS_COMPARISON_PATH", 0.010),
        read_profile("STFR_ACTUATOR_20MS_COMPARISON_PATH", REFERENCE_TAU_S),
        read_profile("STFR_ACTUATOR_50MS_COMPARISON_PATH", 0.050),
    ];
    let reference_index = 2usize;
    let reference = &profiles[reference_index];

    for (profile_index, profile) in profiles.iter().enumerate() {
        assert_eq!(
            profile.training_dataset_sha256, reference.training_dataset_sha256,
            "training dataset digest changed between actuator profiles"
        );
        assert_eq!(
            profile.training_configuration, reference.training_configuration,
            "training plant configuration changed between profiles"
        );
        assert_eq!(
            profile.source_receipt["symtropy_git_sha"], reference.source_receipt["symtropy_git_sha"],
            "Symtropy source SHA changed between profiles"
        );
        assert_eq!(
            profile.source_receipt["symthaea_git_sha"], reference.source_receipt["symthaea_git_sha"],
            "Symthaea source SHA changed between profiles"
        );
        assert_eq!(
            profile.source_receipt["scenario_matrix_sha256"], reference.source_receipt["scenario_matrix_sha256"],
            "scenario matrix digest changed between profiles"
        );
        assert_eq!(
            profile.episodes.len(), reference.episodes.len(),
            "episode count changed in profile {profile_index}"
        );

        for (episode_index, (current, reference_episode)) in
            profile.episodes.iter().zip(reference.episodes.iter()).enumerate()
        {
            assert_eq!(
                current.seed_hex, reference_episode.seed_hex,
                "seed mismatch in profile {profile_index}, episode {episode_index}"
            );
            assert_eq!(
                current.scenario_id, reference_episode.scenario_id,
                "scenario mismatch in profile {profile_index}, episode {episode_index}"
            );
            assert_eq!(
                current.scenario_family, reference_episode.scenario_family,
                "scenario family mismatch in profile {profile_index}, episode {episode_index}"
            );
        }
    }

    let mut evaluation_profiles = Vec::with_capacity(profiles.len());
    for (profile_index, profile) in profiles.iter().enumerate() {
        let mut paired_delta = Map::new();
        let mut relative_degradation = Map::new();
        for (model_index, (model_name, _)) in MODEL_KEYS.iter().enumerate() {
            let deltas = profile.episodes.iter().zip(reference.episodes.iter())
                .map(|(current, baseline)| current.rmse[model_index] - baseline.rmse[model_index])
                .collect::<Vec<_>>();
            let seed = BOOTSTRAP_SEED ^ ((profile_index as u64 + 1) << 8) ^ model_index as u64;
            paired_delta.insert((*model_name).to_owned(), summarize(&deltas, &profile.episodes, seed));
            let reference_mean = model_mean(&reference.model_summary, match *model_name {
                "raw_state_ridge" => "raw_state_ridge_mean_episode_rmse_m_s2",
                "compact_mlp" => "compact_mlp_mean_episode_rmse_m_s2",
                "explicit_history_mlp" => "explicit_history_mlp_mean_episode_rmse_m_s2",
                "persistent_hdc_ltc" => "hdc_ltc_reservoir_mean_episode_rmse_m_s2",
                "ltc_reset_each_step_hdc" => "hdc_ltc_reset_each_step_mean_episode_rmse_m_s2",
                _ => unreachable!(),
            });
            let current_mean = model_mean(&profile.model_summary, match *model_name {
                "raw_state_ridge" => "raw_state_ridge_mean_episode_rmse_m_s2",
                "compact_mlp" => "compact_mlp_mean_episode_rmse_m_s2",
                "explicit_history_mlp" => "explicit_history_mlp_mean_episode_rmse_m_s2",
                "persistent_hdc_ltc" => "hdc_ltc_reservoir_mean_episode_rmse_m_s2",
                "ltc_reset_each_step_hdc" => "hdc_ltc_reset_each_step_mean_episode_rmse_m_s2",
                _ => unreachable!(),
            });
            relative_degradation.insert(
                (*model_name).to_owned(),
                json!((current_mean - reference_mean) / reference_mean.abs().max(EPSILON_RMSE)),
            );
        }

        evaluation_profiles.push(json!({
            "actuator_lag_time_constant_s": profile.evaluation_tau_s,
            "training_dataset_sha256": profile.training_dataset_sha256,
            "model_summary": profile.model_summary,
            "state_memory_ablation": profile.state_memory_ablation,
            "paired_rmse_delta_vs_20ms": paired_delta,
            "relative_rmse_degradation_vs_20ms": relative_degradation
        }));
    }

    let mut resilience_contrasts = Vec::new();
    for (profile_index, profile) in profiles.iter().enumerate() {
        if profile_index == reference_index {
            continue;
        }
        let persistent_vs_history = profile.episodes.iter().zip(reference.episodes.iter())
            .map(|(current, baseline)| {
                (current.rmse[3] - baseline.rmse[3]) - (current.rmse[2] - baseline.rmse[2])
            })
            .collect::<Vec<_>>();
        let persistent_vs_reset = profile.episodes.iter().zip(reference.episodes.iter())
            .map(|(current, baseline)| {
                (current.rmse[3] - baseline.rmse[3]) - (current.rmse[4] - baseline.rmse[4])
            })
            .collect::<Vec<_>>();
        resilience_contrasts.push(json!({
            "actuator_lag_time_constant_s": profile.evaluation_tau_s,
            "persistent_hdc_minus_explicit_history_transfer_delta": summarize(
                &persistent_vs_history, &profile.episodes, BOOTSTRAP_SEED ^ ((profile_index as u64 + 1) << 16) ^ 0x51
            ),
            "persistent_hdc_minus_ltc_reset_each_step_transfer_delta": summarize(
                &persistent_vs_reset, &profile.episodes, BOOTSTRAP_SEED ^ ((profile_index as u64 + 1) << 16) ^ 0xa2
            ),
            "interpretation": "negative means persistent HDC/LTC suffers less additional error than the comparator when moving away from the 20ms evaluation plant"
        }));
    }

    assert_eq!(evaluation_profiles.len(), 4);
    assert_eq!(resilience_contrasts.len(), 3);
    let profile_summaries = evaluation_profiles.iter()
        .map(|profile| {
            let values = profile["relative_rmse_degradation_vs_20ms"].as_object().unwrap();
            assert_eq!(values.len(), MODEL_KEYS.len());
            profile.clone()
        })
        .collect::<Vec<_>>();

    println!("{}", json!({
        "benchmark_id": "symtropy-actuator-lag-transfer-sweep-v0.1",
        "claim_ceiling": "internal_simulator_actuator_lag_generalization_prediction_only_not_real_flight_or_control_evidence",
        "training_actuator_lag_time_constant_s": reference.training_configuration["actuator_lag_time_constant_s"],
        "matched_reference_eval_tau_s": REFERENCE_TAU_S,
        "source_receipt": {
            "symtropy_git_sha": reference.source_receipt["symtropy_git_sha"],
            "symthaea_git_sha": reference.source_receipt["symthaea_git_sha"],
            "scenario_matrix_sha256": reference.source_receipt["scenario_matrix_sha256"],
            "training_dataset_sha256": reference.training_dataset_sha256
        },
        "design": {
            "evaluation_profile_count": profiles.len(),
            "evaluation_time_constants_s": [0.0, 0.010, 0.020, 0.050],
            "episodes_per_profile": reference.episodes.len(),
            "bootstrap_replicates": BOOTSTRAP_REPLICATES,
            "confidence_level": CONFIDENCE_LEVEL,
            "bootstrap_seed_hex": format!("{:016x}", BOOTSTRAP_SEED),
            "bootstrap_unit": "episode",
            "stratification": "resample within the observed scenario-family counts",
            "reference_profile": "20ms evaluation plant",
            "all_episode_seeds_and_scenario_order_match": true,
            "all_training_dataset_digests_match": true,
            "interpretation": "paired per-episode deltas and family-stratified bootstrap intervals are conditional on the smoke scenario families and are not confirmatory evidence for unseen families"
        },
        "evaluation_profiles": profile_summaries,
        "resilience_contrasts": resilience_contrasts,
        "decision_rule": {
            "persistent_memory_claim": "persistent HDC/LTC must outperform both explicit history and the matched LTC-reset-each-step arm in transfer degradation, without hiding worse CVaR90 or worst-episode RMSE",
            "representation_claim": "HDC/LTC must outperform instantaneous and explicit-history models under matched plant profiles",
            "no_score_based_win_required_by_ci": true
        }
    }));
}

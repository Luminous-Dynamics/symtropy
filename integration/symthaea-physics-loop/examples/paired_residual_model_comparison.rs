//! Paired, episode-level comparison across instantaneous, explicit-history and persistent/stateless HDC/LTC models.
//!
//! Episode RMSEs—not per-timestep residuals—are the resampling units. The bootstrap is
//! stratified within observed scenario families, conditioning on this smoke suite's family
//! mix. It does not establish performance on unseen families or a formal safety guarantee.

use std::collections::BTreeMap;
use std::fs;

use serde_json::{json, Value};

const BOOTSTRAP_REPLICATES: usize = 10_000;
const CONFIDENCE_LEVEL: f64 = 0.95;
const BOOTSTRAP_SEED: u64 = 0x5041_4952_4544_0001;

#[derive(Debug, Clone)]
struct EpisodeScore {
    seed_hex: String,
    scenario_id: String,
    scenario_family: String,
    rmse_m_s2: f64,
}

#[derive(Debug, Clone, Copy)]
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

    fn index(&mut self, upper_exclusive: usize) -> usize {
        assert!(upper_exclusive > 0, "cannot sample an empty episode family");
        (self.next_u64() % upper_exclusive as u64) as usize
    }
}

fn read_json_env(name: &str) -> Value {
    let path = std::env::var(name).unwrap_or_else(|_| panic!("{name} must identify a benchmark receipt"));
    let bytes = fs::read(&path).unwrap_or_else(|error| panic!("cannot read {name} at {path}: {error}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|error| panic!("invalid JSON in {name}: {error}"))
}

fn required_string<'a>(value: &'a Value, label: &str) -> &'a str {
    value.as_str().unwrap_or_else(|| panic!("missing string field: {label}"))
}

fn required_usize(value: &Value, label: &str) -> usize {
    value.as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .unwrap_or_else(|| panic!("missing or invalid unsigned integer field: {label}"))
}

fn required_number(value: &Value, label: &str) -> f64 {
    let number = value.as_f64().unwrap_or_else(|| panic!("missing numeric field: {label}"));
    assert!(number.is_finite() && number >= 0.0, "invalid nonnegative finite number: {label}");
    number
}

fn read_episodes(root: &Value, label: &str, steps_per_episode: usize) -> Vec<EpisodeScore> {
    let rows = root["evaluation"]["episode_details"]
        .as_array()
        .unwrap_or_else(|| panic!("{label} lacks per-episode details"));
    assert!(!rows.is_empty(), "{label} has no test episodes");

    rows.iter().enumerate().map(|(index, row)| {
        let invalid = required_usize(&row["invalid_samples"], &format!("{label}[{index}].invalid_samples"));
        let samples = required_usize(&row["sample_count"], &format!("{label}[{index}].sample_count"));
        assert_eq!(invalid, 0, "{label} has invalid predictions at episode {index}");
        assert_eq!(samples, steps_per_episode, "{label} has an incomplete episode at index {index}");

        let seed_hex = required_string(&row["episode_seed_hex"], &format!("{label}[{index}].episode_seed_hex")).to_owned();
        assert_eq!(seed_hex.len(), 16, "{label} episode seed must be 64-bit fixed-width hex");
        assert!(seed_hex.bytes().all(|byte| byte.is_ascii_hexdigit()), "{label} episode seed is not hexadecimal");
        let scenario_id = required_string(&row["scenario_id"], &format!("{label}[{index}].scenario_id")).to_owned();
        let scenario_family = required_string(&row["scenario_family"], &format!("{label}[{index}].scenario_family")).to_owned();
        let rmse = required_number(&row["rmse_m_s2"], &format!("{label}[{index}].rmse_m_s2"));
        EpisodeScore { seed_hex, scenario_id, scenario_family, rmse }
    }).collect()
}

fn ensure_receipts_match(ridge: &Value, hdc: &Value, mlp: &Value, history: &Value) {
    assert_eq!(ridge["benchmark_id"].as_str(), Some("symtropy-causal-transition-residual-ridge-conformal-v0.1"));
    assert_eq!(hdc["benchmark_id"].as_str(), Some("symtropy-hdc-ltc-residual-reservoir-heldout-v0.1"));
    assert_eq!(mlp["benchmark_id"].as_str(), Some("symtropy-compact-mlp-residual-heldout-v0.1"));
    assert_eq!(history["benchmark_id"].as_str(), Some("symtropy-explicit-history-mlp-residual-heldout-v0.1"));

    for field in [
        "symtropy_git_sha",
        "symthaea_git_sha",
        "rustc_version",
        "scenario_matrix_sha256",
        "training_dataset_sha256",
    ] {
        let r = required_string(&ridge["source_receipt"][field], &format!("ridge.source_receipt.{field}"));
        let h = required_string(&hdc["source_receipt"][field], &format!("hdc.source_receipt.{field}"));
        let m = required_string(&mlp["source_receipt"][field], &format!("mlp.source_receipt.{field}"));
        let w = required_string(&history["source_receipt"][field], &format!("history.source_receipt.{field}"));
        assert_eq!(r, h, "model arms do not share source receipt field {field}");
        assert_eq!(r, m, "model arms do not share source receipt field {field}");
        assert_eq!(r, w, "model arms do not share source receipt field {field}");
    }

    let ridge_eval = &ridge["evaluation"];
    let hdc_eval = &hdc["evaluation"];
    let mlp_eval = &mlp["evaluation"];
    let history_eval = &history["evaluation"];
    assert_eq!(required_string(&ridge_eval["split"], "ridge split"), "test");
    assert_eq!(required_string(&hdc_eval["split"], "hdc split"), "test");
    assert_eq!(required_string(&mlp_eval["split"], "mlp split"), "test");
    assert_eq!(required_string(&history_eval["split"], "history split"), "test");
    for field in ["episodes_requested", "steps_per_episode"] {
        let expected = required_usize(&ridge_eval[field], &format!("ridge.evaluation.{field}"));
        assert_eq!(required_usize(&hdc_eval[field], &format!("hdc.evaluation.{field}")), expected, "test horizon mismatch");
        assert_eq!(required_usize(&mlp_eval[field], &format!("mlp.evaluation.{field}")), expected, "test horizon mismatch");
        assert_eq!(required_usize(&history_eval[field], &format!("history.evaluation.{field}")), expected, "test horizon mismatch");
    }
    let stateless_eval = &hdc["stateless_state_ablation"]["evaluation"];
    assert_eq!(required_string(&stateless_eval["split"], "stateless HDC split"), "test");
    assert_eq!(required_string(&hdc["stateless_state_ablation"]["claim_ceiling"], "stateless HDC claim ceiling"),
        "HDC_encoder_with_encoder_derivative_history_but_no_persistent_LTC_recurrent_state_one_step_prediction_only");
    assert_eq!(required_string(&hdc["stateless_state_ablation"]["state_policy"], "stateless HDC state policy"),
        "reset LTC network immediately before each transition; preserve HDC encoder derivative history within episode");
    assert_eq!(hdc["stateless_state_ablation"]["training"]["same_transition_targets_as_persistent_arm"], true);
    for field in ["episodes_requested", "steps_per_episode"] {
        assert_eq!(required_usize(&stateless_eval[field], &format!("stateless.evaluation.{field}")),
            required_usize(&ridge_eval[field], &format!("ridge.evaluation.{field}")), "stateless horizon mismatch");
    }
    assert_eq!(ridge_eval["scenario_ids"], hdc_eval["test_scenario_ids"], "HDC test scenario catalogue mismatch");
    assert_eq!(ridge_eval["scenario_ids"], mlp_eval["test_scenario_ids"], "MLP test scenario catalogue mismatch");
    assert_eq!(ridge_eval["scenario_ids"], history_eval["test_scenario_ids"], "history test scenario catalogue mismatch");
    assert_eq!(ridge_eval["scenario_ids"], stateless_eval["test_scenario_ids"], "stateless HDC scenario catalogue mismatch");
    assert_eq!(ridge_eval["nominal_model"], hdc_eval["nominal_model"], "HDC nominal-model parameters mismatch");
    assert_eq!(ridge_eval["nominal_model"], mlp_eval["nominal_model"], "MLP nominal-model parameters mismatch");
    assert_eq!(ridge_eval["nominal_model"], history_eval["nominal_model"], "history nominal-model parameters mismatch");
    assert_eq!(ridge_eval["nominal_model"], stateless_eval["nominal_model"], "stateless HDC nominal-model parameters mismatch");
}

fn grouped_indices(episodes: &[EpisodeScore]) -> BTreeMap<&str, Vec<usize>> {
    let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, episode) in episodes.iter().enumerate() {
        groups.entry(episode.scenario_family.as_str()).or_default().push(index);
    }
    groups
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0usize), |(sum, count), value| (sum + value, count + 1));
    assert!(count > 0, "cannot average an empty collection");
    sum / count as f64
}

fn cvar90(episodes: &[EpisodeScore]) -> f64 {
    let mut errors = episodes.iter().map(|episode| episode.rmse_m_s2).collect::<Vec<_>>();
    errors.sort_by(f64::total_cmp);
    let tail_count = ((errors.len() as f64 * 0.10).ceil() as usize).max(1);
    mean(errors.iter().rev().take(tail_count).copied())
}

fn percentile(sorted_values: &[f64], probability: f64) -> f64 {
    assert!(!sorted_values.is_empty(), "cannot take percentile of empty values");
    let position = probability * (sorted_values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let weight = position - lower as f64;
    sorted_values[lower] * (1.0 - weight) + sorted_values[upper] * weight
}

fn paired_stratified_bootstrap(
    candidate: &[EpisodeScore],
    reference: &[EpisodeScore],
    pair_seed: u64,
) -> (f64, f64, f64) {
    assert_eq!(candidate.len(), reference.len(), "paired arrays differ in length");
    let groups = grouped_indices(reference);
    let point = mean(
        candidate.iter().zip(reference.iter()).map(|(candidate, reference)| {
            assert_eq!(candidate.seed_hex, reference.seed_hex);
            assert_eq!(candidate.scenario_id, reference.scenario_id);
            assert_eq!(candidate.scenario_family, reference.scenario_family);
            candidate.rmse_m_s2 - reference.rmse_m_s2
        }),
    );
    let mut rng = SplitMix64::new(BOOTSTRAP_SEED ^ pair_seed);
    let mut bootstrap_means = Vec::with_capacity(BOOTSTRAP_REPLICATES);

    for _ in 0..BOOTSTRAP_REPLICATES {
        let mut delta_sum = 0.0;
        let mut draw_count = 0usize;
        // Preserve the observed scenario-family mix. Resample independent episodes
        // within each family rather than treating highly correlated frames as IID.
        for indices in groups.values() {
            for _ in 0..indices.len() {
                let index = indices[rng.index(indices.len())];
                delta_sum += candidate[index].rmse_m_s2 - reference[index].rmse_m_s2;
                draw_count += 1;
            }
        }
        assert_eq!(draw_count, candidate.len());
        bootstrap_means.push(delta_sum / draw_count as f64);
    }

    bootstrap_means.sort_by(f64::total_cmp);
    let alpha = (1.0 - CONFIDENCE_LEVEL) / 2.0;
    (
        point,
        percentile(&bootstrap_means, alpha),
        percentile(&bootstrap_means, 1.0 - alpha),
    )
}

fn paired_summary(
    candidate_name: &str,
    reference_name: &str,
    candidate: &[EpisodeScore],
    reference: &[EpisodeScore],
    pair_seed: u64,
) -> Value {
    let (delta, low, high) = paired_stratified_bootstrap(candidate, reference, pair_seed);
    let reference_mean = mean(reference.iter().map(|episode| episode.rmse_m_s2));
    json!({
        "candidate_model": candidate_name,
        "reference_model": reference_name,
        "mean_paired_delta_rmse_m_s2": delta,
        "stratified_episode_bootstrap_95_ci_m_s2": [low, high],
        "negative_delta_means_candidate_has_lower_episode_rmse": true,
        "relative_mean_rmse_change_percent": if reference_mean > 0.0 {
            Some(100.0 * delta / reference_mean)
        } else { None }
    })
}

fn family_summaries(
    ridge: &[EpisodeScore],
    hdc: &[EpisodeScore],
    mlp: &[EpisodeScore],
    history: &[EpisodeScore],
    stateless_hdc: &[EpisodeScore],
) -> Vec<Value> {
    let groups = grouped_indices(ridge);
    groups.into_iter().map(|(family, indices)| {
        let r = mean(indices.iter().map(|index| ridge[*index].rmse_m_s2));
        let h = mean(indices.iter().map(|index| hdc[*index].rmse_m_s2));
        let m = mean(indices.iter().map(|index| mlp[*index].rmse_m_s2));
        let w = mean(indices.iter().map(|index| history[*index].rmse_m_s2));
        let s = mean(indices.iter().map(|index| stateless_hdc[*index].rmse_m_s2));
        json!({
            "scenario_family": family,
            "episode_count": indices.len(),
            "raw_state_ridge_mean_episode_rmse_m_s2": r,
            "hdc_ltc_reservoir_mean_episode_rmse_m_s2": h,
            "compact_mlp_mean_episode_rmse_m_s2": m,
            "explicit_history_mlp_mean_episode_rmse_m_s2": w,
            "stateless_hdc_ltc_mean_episode_rmse_m_s2": s,
            "hdc_minus_ridge_m_s2": h-r,
            "mlp_minus_ridge_m_s2": m-r,
            "history_minus_ridge_m_s2": w-r,
            "stateless_hdc_minus_ridge_m_s2": s-r,
            "hdc_minus_mlp_m_s2": h-m,
            "hdc_minus_history_m_s2": h-w,
            "history_minus_mlp_m_s2": w-m,
            "persistent_hdc_minus_stateless_hdc_m_s2": h-s
        })
    }).collect()
}

fn main() {
    let ridge_root = read_json_env("STFR_RIDGE_OUTPUT_PATH");
    let hdc_root = read_json_env("STFR_HDC_OUTPUT_PATH");
    let mlp_root = read_json_env("STFR_MLP_OUTPUT_PATH");
    let history_root = read_json_env("STFR_HISTORY_MLP_OUTPUT_PATH");
    ensure_receipts_match(&ridge_root, &hdc_root, &mlp_root, &history_root);

    let mut stateless_hdc_root = hdc_root.clone();
    stateless_hdc_root["evaluation"] = hdc_root["stateless_state_ablation"]["evaluation"].clone();

    let episodes_requested = required_usize(&ridge_root["evaluation"]["episodes_requested"], "ridge episodes");
    let steps_per_episode = required_usize(&ridge_root["evaluation"]["steps_per_episode"], "ridge steps");
    for (label, root) in [("hdc", &hdc_root), ("mlp", &mlp_root), ("history", &history_root), ("stateless_hdc", &stateless_hdc_root)] {
        assert_eq!(episodes_requested, required_usize(&root["evaluation"]["episodes_requested"], &format!("{label} episodes")));
    }

    let ridge = read_episodes(&ridge_root, "ridge", steps_per_episode);
    let hdc = read_episodes(&hdc_root, "hdc", steps_per_episode);
    let mlp = read_episodes(&mlp_root, "mlp", steps_per_episode);
    let history = read_episodes(&history_root, "history", steps_per_episode);
    let stateless_hdc = read_episodes(&stateless_hdc_root, "stateless_hdc", steps_per_episode);
    for (label, rows) in [("ridge", &ridge), ("hdc", &hdc), ("mlp", &mlp), ("history", &history), ("stateless_hdc", &stateless_hdc)] {
        assert_eq!(rows.len(), episodes_requested, "{label} episode count mismatch");
    }

    for index in 0..episodes_requested {
        for (label, other) in [
            ("hdc", &hdc[index]),
            ("mlp", &mlp[index]),
            ("history", &history[index]),
            ("stateless_hdc", &stateless_hdc[index]),
        ] {
            assert_eq!(ridge[index].seed_hex, other.seed_hex, "seed mismatch in {label} episode {index}");
            assert_eq!(ridge[index].scenario_id, other.scenario_id, "scenario mismatch in {label} episode {index}");
            assert_eq!(ridge[index].scenario_family, other.scenario_family, "family mismatch in {label} episode {index}");
        }
    }

    let shared_receipt = &ridge_root["source_receipt"];
    println!("{}", json!({
        "comparison_id": "symtropy-residual-model-paired-comparison-v0.1",
        "claim_ceiling": "paired_episode_level_one_step_prediction_comparison_conditional_on_smoke_scenario_families",
        "source_receipt": {
            "symtropy_git_sha": shared_receipt["symtropy_git_sha"],
            "symthaea_git_sha": shared_receipt["symthaea_git_sha"],
            "rustc_version": shared_receipt["rustc_version"],
            "scenario_matrix_sha256": shared_receipt["scenario_matrix_sha256"],
            "training_dataset_sha256": shared_receipt["training_dataset_sha256"]
        },
        "design": {
            "episodes": episodes_requested,
            "steps_per_episode": steps_per_episode,
            "bootstrap_replicates": BOOTSTRAP_REPLICATES,
            "confidence_level": CONFIDENCE_LEVEL,
            "bootstrap_seed_hex": format!("{:016x}", BOOTSTRAP_SEED),
            "pair_seed_scheme": "bootstrap_seed XOR pair identifiers 1 through 10",
            "bootstrap_unit": "episode",
            "stratification": "resample episodes with replacement within each observed scenario family",
            "seed_hex_equality_required": true,
            "scenario_order_equality_required": true,
            "invalid_episode_policy": "fail closed",
            "interpretation": "Intervals condition on the observed scenario-family mix. With only a smoke-sized sample and three families, they are engineering diagnostics, not confirmatory evidence of generalization to unseen families."
        },
        "model_summary": {
            "raw_state_ridge_mean_episode_rmse_m_s2": mean(ridge.iter().map(|episode| episode.rmse_m_s2)),
            "raw_state_ridge_cvar90_episode_rmse_m_s2": cvar90(&ridge),
            "raw_state_ridge_worst_episode_rmse_m_s2": ridge.iter().map(|episode| episode.rmse_m_s2).max_by(f64::total_cmp).unwrap(),
            "compact_mlp_mean_episode_rmse_m_s2": mean(mlp.iter().map(|episode| episode.rmse_m_s2)),
            "compact_mlp_cvar90_episode_rmse_m_s2": cvar90(&mlp),
            "compact_mlp_worst_episode_rmse_m_s2": mlp.iter().map(|episode| episode.rmse_m_s2).max_by(f64::total_cmp).unwrap(),
            "hdc_ltc_reservoir_mean_episode_rmse_m_s2": mean(hdc.iter().map(|episode| episode.rmse_m_s2)),
            "hdc_ltc_reservoir_cvar90_episode_rmse_m_s2": cvar90(&hdc),
            "hdc_ltc_reservoir_worst_episode_rmse_m_s2": hdc.iter().map(|episode| episode.rmse_m_s2).max_by(f64::total_cmp).unwrap(),
            "explicit_history_mlp_mean_episode_rmse_m_s2": mean(history.iter().map(|episode| episode.rmse_m_s2)),
            "explicit_history_mlp_cvar90_episode_rmse_m_s2": cvar90(&history),
            "explicit_history_mlp_worst_episode_rmse_m_s2": history.iter().map(|episode| episode.rmse_m_s2).max_by(f64::total_cmp).unwrap(),
            "stateless_hdc_ltc_mean_episode_rmse_m_s2": mean(stateless_hdc.iter().map(|episode| episode.rmse_m_s2)),
            "stateless_hdc_ltc_cvar90_episode_rmse_m_s2": cvar90(&stateless_hdc),
            "stateless_hdc_ltc_worst_episode_rmse_m_s2": stateless_hdc.iter().map(|episode| episode.rmse_m_s2).max_by(f64::total_cmp).unwrap()
        },
        "state_memory_ablation": {
            "policy": "reset LTC recurrent state before every step while preserving encoder derivative history",
            "question": "Does persistent LTC recurrent state help beyond the encoder's temporal derivative features?",
            "persistent_hdc_mean_episode_rmse_m_s2": mean(hdc.iter().map(|episode| episode.rmse_m_s2)),
            "reset_each_step_hdc_mean_episode_rmse_m_s2": mean(stateless_hdc.iter().map(|episode| episode.rmse_m_s2)),
            "persistent_minus_reset_each_step_m_s2": mean(hdc.iter().zip(stateless_hdc.iter()).map(|(persistent, stateless)| persistent.rmse_m_s2-stateless.rmse_m_s2))
        },
        "paired_comparisons": [
            paired_summary("hdc_ltc_reservoir", "raw_state_ridge", &hdc, &ridge, 0x01),
            paired_summary("compact_mlp", "raw_state_ridge", &mlp, &ridge, 0x02),
            paired_summary("hdc_ltc_reservoir", "compact_mlp", &hdc, &mlp, 0x03),
            paired_summary("explicit_history_mlp", "raw_state_ridge", &history, &ridge, 0x04),
            paired_summary("explicit_history_mlp", "compact_mlp", &history, &mlp, 0x05),
            paired_summary("hdc_ltc_reservoir", "explicit_history_mlp", &hdc, &history, 0x06),
            paired_summary("stateless_hdc_ltc", "raw_state_ridge", &stateless_hdc, &ridge, 0x07),
            paired_summary("stateless_hdc_ltc", "compact_mlp", &stateless_hdc, &mlp, 0x08),
            paired_summary("stateless_hdc_ltc", "explicit_history_mlp", &stateless_hdc, &history, 0x09),
            paired_summary("hdc_ltc_reservoir", "stateless_hdc_ltc", &hdc, &stateless_hdc, 0x0a)
        ],
        "family_summary": family_summaries(&ridge, &hdc, &mlp, &history, &stateless_hdc),
        "per_episode": (0..episodes_requested).map(|index| json!({
            "episode_index": index,
            "episode_seed_hex": ridge[index].seed_hex,
            "scenario_id": ridge[index].scenario_id,
            "scenario_family": ridge[index].scenario_family,
            "raw_state_ridge_rmse_m_s2": ridge[index].rmse_m_s2,
            "compact_mlp_rmse_m_s2": mlp[index].rmse_m_s2,
            "explicit_history_mlp_rmse_m_s2": history[index].rmse_m_s2,
            "hdc_ltc_reservoir_rmse_m_s2": hdc[index].rmse_m_s2,
            "stateless_hdc_ltc_rmse_m_s2": stateless_hdc[index].rmse_m_s2
        })).collect::<Vec<_>>()
    }));
}

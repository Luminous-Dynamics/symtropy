#!/usr/bin/env bash
set -euo pipefail

: "$SYMTROPY_PATH"
: "$SYMTHAEA_PATH"
test -n "$SYMTROPY_PATH"
test -n "$SYMTHAEA_PATH"

brain_manifest="$SYMTROPY_PATH/crates/bridges/symthaea-bevy-brain/Cargo.toml"
robot_manifest="$SYMTROPY_PATH/crates/bridges/symtropy-robotics-bridge/Cargo.toml"
integration_manifest="$SYMTROPY_PATH/integration/symthaea-physics-loop/Cargo.toml"
integration_lock="$SYMTROPY_PATH/integration/symthaea-physics-loop/Cargo.lock"
backup_dir="$(mktemp -d)"
test -f "$brain_manifest"
test -f "$robot_manifest"
test -f "$integration_manifest"
test -f "$SYMTHAEA_PATH/Cargo.toml"
test -f "$SYMTHAEA_PATH/src/cognitive_loop/mod.rs"

# Embed exact experiment inputs into the machine-readable receipt.
export STFR_SYMTROPY_SHA="$(git -C "$SYMTROPY_PATH" rev-parse HEAD)"
export STFR_SYMTHAEA_SHA="$(git -C "$SYMTHAEA_PATH" rev-parse HEAD)"
export STFR_RUSTC_VERSION="$(rustc --version)"
export STFR_SCENARIO_MATRIX_SHA256="$(sha256sum "$SYMTROPY_PATH/integration/symthaea-physics-loop/src/scenario_matrix.rs" | awk '{print $1}')"

cp "$brain_manifest" "$backup_dir/brain.toml"
cp "$robot_manifest" "$backup_dir/robotics.toml"
if [ -f "$integration_lock" ]; then
  cp "$integration_lock" "$backup_dir/integration.lock"
  touch "$backup_dir/had-lock"
fi

restore() {
  cp "$backup_dir/brain.toml" "$brain_manifest"
  cp "$backup_dir/robotics.toml" "$robot_manifest"
  if [ -f "$backup_dir/had-lock" ]; then
    cp "$backup_dir/integration.lock" "$integration_lock"
  else
    rm -f "$integration_lock"
  fi
  rm -rf "$backup_dir"
}
trap restore EXIT

# Temporarily bypass the standalone stubs and resolve shared Symthaea types
# from one exact source checkout, avoiding registry/path type duplication.
sed -i 's#path = "../../../stubs/symthaea"#path = "../../../../symthaea"#' "$brain_manifest"
sed -i 's#symthaea-fep = "0.1.0"#symthaea-fep = { path = "../../../../symthaea/crates/core/symthaea-fep" }#' "$robot_manifest"
sed -i 's#symthaea-core = "0.5.1"#symthaea-core = { path = "../../../../symthaea/crates/core/symthaea-core" }#' "$robot_manifest"
sed -i 's#symthaea-consciousness-equation = "0.1.0"#symthaea-consciousness-equation = { path = "../../../../symthaea/crates/domains/symthaea-consciousness-equation" }#' "$robot_manifest"

grep -q 'path = "../../../../symthaea"' "$brain_manifest"
grep -q 'path = "../../../../symthaea/crates/core/symthaea-fep"' "$robot_manifest"
grep -q 'path = "../../../../symthaea/crates/core/symthaea-core"' "$robot_manifest"
grep -q 'path = "../../../../symthaea/crates/domains/symthaea-consciousness-equation"' "$robot_manifest"

cargo test --manifest-path "$integration_manifest" --lib --tests -- --nocapture
if [ "${STFB_RUN_BENCHMARK:-0}" = "1" ]; then
  out="${STFB_OUTPUT_PATH:-$SYMTROPY_PATH/symthaea-symtropy-flight-transfer.json}"
  cargo run --manifest-path "$integration_manifest" --example paired_flight_transfer > "$out"
  jq -e '
    .benchmark_id == "symthaea-symtropy-flight-transfer-v0.1"
    and .evaluation.matched_seeds == true
    and .paired_summary.paired_trial_count == .evaluation.trials
    and (.trials | length) == .evaluation.trials
    and (.trials | length) >= 1
    and .trials[0].controller.pid_baseline != null
    and .trials[0].controller.symthaea_trained_no_online_fep_native_backend != null
    and .paired_summary.sim_to_sim_transfer_gap_no_online_fep.bootstrap_95_ci_rmse_gap_m != null
    and all(.trials[];
      .controller.pd_baseline.invalid_inputs == 0
      and .controller.pid_baseline.invalid_inputs == 0
      and .controller.symthaea_untrained_no_fep_ablation.invalid_inputs == 0
      and .controller.symthaea_trained_no_online_fep.invalid_inputs == 0
      and .controller.symthaea_trained_with_fep.invalid_inputs == 0
      and .controller.symthaea_trained_no_online_fep_native_backend.invalid_inputs == null
    )
  ' "$out"
fi

if [ "${STFR_RUN_BENCHMARK:-0}" = "1" ]; then
  out="${STFR_OUTPUT_PATH:-$SYMTROPY_PATH/symthaea-symtropy-bounded-residual-matrix.json}"
  cargo run --manifest-path "$integration_manifest" --example bounded_residual_transfer > "$out"
  jq -e '
    .benchmark_id == "symthaea-symtropy-bounded-residual-matrix-v0.2"
    and .evaluation.matched_seed_across_authority_arms == true
    and .evaluation.scenario_count == 5
    and (.evaluation.authorities | length) == 6
    and (.source_receipt.symtropy_git_sha | test("^[0-9a-f]{40}$"))
    and .source_receipt.symthaea_git_sha == "bf787f4d655bd85f084cf75990232f19f7d809a1"
    and (.source_receipt.rustc_version | startswith("rustc "))
    and (.source_receipt.scenario_matrix_sha256 | test("^[0-9a-f]{64}$"))
    and (.risk_summary_by_authority | length) == 6
    and all(.risk_summary_by_authority[];
      .empirical_cvar90_rmse_m != null
      and .worst_observed_rmse_m != null
      and .ground_contact_episode_rate >= 0
      and .invalid_input_total == 0
    )
    and (.paired_summary_by_scenario | length) == .evaluation.scenario_count
    and (.trials | length) == .evaluation.scenario_trial_pair_count
    and (.trials | length) == (.evaluation.scenario_count * .evaluation.trials_per_scenario)
    and all(.paired_summary_by_scenario[];
      (.paired_summary_vs_pid | length) == 6
      and .paired_summary_vs_pid[0].authority == 0
      and .paired_summary_vs_pid[0].mean_delta_rmse_vs_pid_m == 0
      and .paired_summary_vs_pid[0].mean_delta_within_5cm_fraction_vs_pid == 0
      and .paired_summary_vs_pid[0].bootstrap_95_ci_delta_rmse_vs_pid_m != null
    )
    and all(.trials[];
      (.arms | length) == 6
      and .matched_seed_across_authorities == true
      and .arms[0].authority == 0
      and .arms[0].guard_rejections == 0
      and .arms[0].residual_clipped_steps == 0
      and all(.arms[];
        .guard_rejections == 0
        and .invalid_inputs == 0
        and .rmse_m != null
        and .within_5cm_fraction != null
        and .max_position_error_m != null
        and .controller_compute_p95_us != null
      )
    )
  ' "$out"
fi

if [ "${STFR_RUN_BENCHMARK:-0}" = "1" ]; then
  out="${STFR_DATASET_OUTPUT_PATH:-$SYMTROPY_PATH/symtropy-causal-transition-residual-dataset.json}"
  cargo run --manifest-path "$integration_manifest" --example causal_transition_dataset > "$out"
  jq -e '
    .benchmark_id == "symtropy-causal-transition-residual-dataset-v0.1"
    and .claim_ceiling == "training_split_transition_dataset_and_target_plumbing_only"
    and (.source_receipt.symtropy_git_sha | test("^[0-9a-f]{40}$"))
    and .source_receipt.symthaea_git_sha == "bf787f4d655bd85f084cf75990232f19f7d809a1"
    and (.source_receipt.rustc_version | startswith("rustc "))
    and (.source_receipt.scenario_matrix_sha256 | test("^[0-9a-f]{64}$"))
    and .collection.invalid_target_count == 0
    and .collection.sample_count == (.samples | length)
    and .collection.sample_count == (.collection.episodes * .collection.steps_per_episode)
    and .collection.training_randomization.force_values_exported_per_sample == false
    and .collection.training_randomization.force_labels_available_to_target_extractor == false
    and (.collection.nominal_model_id == "linear-thrust-gravity-linear-drag-v0.1")
    and (.collection.nominal_model.mass_kg > 0)
    and (.collection.nominal_model.gravity_m_s2 > 0)
    and (.collection.nominal_model.linear_drag_n_s_per_m >= 0)
    and .collection.training_randomization.design == "crossed_scenario_by_disturbance_family"
    and .collection.training_randomization.complete_factorial_design == true
    and (.collection.training_randomization.scenario_by_disturbance_family_episode_counts | length) == 4
    and all(.collection.training_randomization.scenario_by_disturbance_family_episode_counts[]; all(.[]; . > 0))
    and (.collection.training_randomization.episode_family_counts | length) == 4
    and all(.collection.training_randomization.episode_family_counts[]; . >= 1)
    and all(.samples[];
      .split == "train"
      and .data_schema_version == "causal-transition-residual-row-v0.1"
      and .residual_target_algorithm_version == "observed-minus-nominal-world-acceleration-v0.1"
      and .nominal_model_id == "linear-thrust-gravity-linear-drag-v0.1"
      and .episode_seed != null
      and .scenario_family != null
      and .dt_s > 0
      and (.target.residual_acceleration_world_m_s2 | length) == 3
      and (.applied_action_ctrl | length) == 4
      and (.state_t.timestamp_s < .state_t1.timestamp_s)
    )
  ' "$out"
fi

if [ "${STFR_RUN_BENCHMARK:-0}" = "1" ]; then
  dataset_input="${STFR_DATASET_OUTPUT_PATH:-$SYMTROPY_PATH/symtropy-causal-transition-residual-dataset.json}"
  test -s "$dataset_input"
  export STFR_DATASET_INPUT_PATH="$dataset_input"
  export STFR_TRAIN_DATASET_SHA256="$(sha256sum "$dataset_input" | awk '{print $1}')"
  ridge_out="${STFR_RIDGE_OUTPUT_PATH:-$SYMTROPY_PATH/symtropy-causal-residual-ridge-conformal-heldout.json}"
  cargo run --manifest-path "$integration_manifest" --example causal_residual_ridge_benchmark > "$ridge_out"
  jq -e '
    .benchmark_id == "symtropy-causal-transition-residual-ridge-conformal-v0.1"
    and .claim_ceiling == "one_step_heldout_prediction_and_empirical_episode_block_calibration_not_control_or_safety_evidence"
    and (.source_receipt.symtropy_git_sha | test("^[0-9a-f]{40}$"))
    and .source_receipt.symthaea_git_sha == "bf787f4d655bd85f084cf75990232f19f7d809a1"
    and (.source_receipt.rustc_version | startswith("rustc "))
    and (.source_receipt.scenario_matrix_sha256 | test("^[0-9a-f]{64}$"))
    and (.source_receipt.training_dataset_sha256 | test("^[0-9a-f]{64}$"))
    and .training.sample_count > 0
    and .training.training_seconds >= 0
    and .training.hidden_force_labels_used_as_features == false
    and .calibration.split == "validation"
    and .calibration.alpha == 0.1
    and .calibration.valid_score_count == .calibration.score_count_requested
    and .calibration.valid_score_count >= 9
    and (.calibration.conformal_radius_m_s2 != null)
    and all(.calibration.scores_m_s2[]; . >= 0)
    and .evaluation.split == "test"
    and .evaluation.summary.metrics.sample_count == (.evaluation.episodes_requested * .evaluation.steps_per_episode)
    and .evaluation.summary.metrics.invalid_samples == 0
    and .evaluation.summary.invalid_episode_count == 0
    and .evaluation.summary.metrics.ridge_residual_rmse_m_s2 >= 0
    and .evaluation.summary.metrics.zero_residual_physics_only_rmse_m_s2 >= 0
    and .evaluation.summary.metrics.inference_p50_us != null
    and .evaluation.summary.metrics.inference_p95_us != null
    and .evaluation.summary.metrics.inference_p99_us != null
    and .evaluation.summary.conformal_coverage.radius_euclidean_m_s2 != null
    and .evaluation.summary.conformal_coverage.episode_coverage >= 0
    and .evaluation.summary.conformal_coverage.episode_coverage <= 1
    and (.evaluation.summary.family_metrics | length) == 3
    and all(.evaluation.summary.family_metrics[]; .sample_count > 0 and .invalid_samples == 0 and .episode_coverage != null and .inference_p95_us != null)
  ' "$ridge_out"
fi

if [ "${STFR_RUN_BENCHMARK:-0}" = "1" ]; then
  dataset_input="${STFR_DATASET_OUTPUT_PATH:-$SYMTROPY_PATH/symtropy-causal-transition-residual-dataset.json}"
  test -s "$dataset_input"
  export STFR_DATASET_INPUT_PATH="$dataset_input"
  export STFR_TRAIN_DATASET_SHA256="$(sha256sum "$dataset_input" | awk '{print $1}')"
  hdc_out="${STFR_HDC_OUTPUT_PATH:-$SYMTROPY_PATH/symtropy-hdc-ltc-residual-reservoir-heldout.json}"
  cargo run --manifest-path "$integration_manifest" --example hdc_ltc_residual_reservoir_benchmark > "$hdc_out"
  jq -e '
    .benchmark_id == "symtropy-hdc-ltc-residual-reservoir-heldout-v0.1"
    and .claim_ceiling == "frozen_hdc_ltc_reservoir_readout_one_step_prediction_only"
    and (.source_receipt.symtropy_git_sha | test("^[0-9a-f]{40}$"))
    and .source_receipt.symthaea_git_sha == "bf787f4d655bd85f084cf75990232f19f7d809a1"
    and (.source_receipt.rustc_version | startswith("rustc "))
    and (.source_receipt.scenario_matrix_sha256 | test("^[0-9a-f]{64}$"))
    and (.source_receipt.training_dataset_sha256 | test("^[0-9a-f]{64}$"))
    and .representation.hdc_dimension == 16384
    and .representation.reservoir == "HdcLtcUnifiedNetwork"
    and .representation.reservoir_weights_frozen == true
    and .representation.trainable_weights == "readout only; no end-to-end HDC-LTC BPTT"
    and .representation.readout_feature_dimension == 262
    and .training.sample_count > 0
    and .training.readout_fit_seconds >= 0
    and .training.hidden_force_labels_used_as_features == false
    and .evaluation.split == "test"
    and .evaluation.sample_count == (.evaluation.episodes_requested * .evaluation.steps_per_episode)
    and .evaluation.invalid_samples == 0
    and .evaluation.metrics.invalid_samples == 0
    and .evaluation.metrics.hdc_ltc_reservoir_residual_rmse_m_s2 >= 0
    and .evaluation.metrics.zero_residual_physics_only_rmse_m_s2 >= 0
    and (.evaluation.family_metrics | length) == 3
    and all(.evaluation.family_metrics[]; .sample_count > 0 and .invalid_samples == 0 and .inference_p95_us != null)
  ' "$hdc_out"

  mlp_out="${STFR_MLP_OUTPUT_PATH:-$SYMTROPY_PATH/symtropy-compact-mlp-residual-heldout.json}"
  cargo run --manifest-path "$integration_manifest" --example compact_mlp_residual_benchmark > "$mlp_out"
  jq -e '
    .benchmark_id == "symtropy-compact-mlp-residual-heldout-v0.1"
    and .claim_ceiling == "compact_mlp_one_step_heldout_prediction_only_no_control_or_safety_claim"
    and (.source_receipt.symtropy_git_sha | test("^[0-9a-f]{40}$"))
    and .source_receipt.symthaea_git_sha == "bf787f4d655bd85f084cf75990232f19f7d809a1"
    and (.source_receipt.scenario_matrix_sha256 | test("^[0-9a-f]{64}$"))
    and (.source_receipt.training_dataset_sha256 | test("^[0-9a-f]{64}$"))
    and .representation.input_features == 18
    and .representation.hidden_units == 16
    and .representation.parameter_count == 355
    and .representation.temporal_state == false
    and .training.sample_count > 0
    and .training.hidden_force_labels_used_as_features == false
    and .training.final_training_rmse_m_s2 >= 0
    and .evaluation.split == "test"
    and .evaluation.sample_count == (.evaluation.episodes_requested * .evaluation.steps_per_episode)
    and .evaluation.invalid_samples == 0
    and .evaluation.metrics.invalid_samples == 0
    and .evaluation.metrics.compact_mlp_residual_rmse_m_s2 >= 0
    and .evaluation.metrics.zero_residual_physics_only_rmse_m_s2 >= 0
    and (.evaluation.family_metrics | length) == 3
    and all(.evaluation.family_metrics[]; .sample_count > 0 and .invalid_samples == 0 and .inference_p95_us != null)
    and (.evaluation.episode_details | length) == .evaluation.episodes_requested
    and all(.evaluation.episode_details[]; .sample_count > 0 and .invalid_samples == 0 and .inference_p95_us != null)
  ' "$mlp_out"

  history_out="${STFR_HISTORY_MLP_OUTPUT_PATH:-$SYMTROPY_PATH/symtropy-explicit-history-mlp-residual-heldout.json}"
  cargo run --manifest-path "$integration_manifest" --example history_window_mlp_residual_benchmark > "$history_out"
  jq -e '
    .benchmark_id == "symtropy-explicit-history-mlp-residual-heldout-v0.1"
    and .claim_ceiling == "explicit_history_mlp_one_step_heldout_prediction_only_no_control_or_safety_claim"
    and (.source_receipt.symtropy_git_sha | test("^[0-9a-f]{40}$"))
    and .source_receipt.symthaea_git_sha == "bf787f4d655bd85f084cf75990232f19f7d809a1"
    and (.source_receipt.scenario_matrix_sha256 | test("^[0-9a-f]{64}$"))
    and (.source_receipt.training_dataset_sha256 | test("^[0-9a-f]{64}$"))
    and .representation.input_features_per_transition == 18
    and .representation.history_depth_including_current == 5
    and .representation.historical_lags == 4
    and .representation.input_feature_dimension == 94
    and .representation.historical_lag_mask_count == 4
    and .representation.hidden_units == 16
    and .representation.parameter_count == 1571
    and .representation.temporal_representation == "explicit four-step causal delay embedding; no recurrent hidden state"
    and .training.sample_count > 0
    and .training.hidden_force_labels_used_as_features == false
    and .training.final_training_rmse_m_s2 >= 0
    and .evaluation.split == "test"
    and .evaluation.sample_count == (.evaluation.episodes_requested * .evaluation.steps_per_episode)
    and .evaluation.invalid_samples == 0
    and .evaluation.metrics.invalid_samples == 0
    and .evaluation.metrics.history_window_mlp_residual_rmse_m_s2 >= 0
    and .evaluation.metrics.zero_residual_physics_only_rmse_m_s2 >= 0
    and (.evaluation.family_metrics | length) == 3
    and all(.evaluation.family_metrics[]; .sample_count > 0 and .invalid_samples == 0 and .inference_p95_us != null)
    and (.evaluation.episode_details | length) == .evaluation.episodes_requested
    and all(.evaluation.episode_details[]; .sample_count > 0 and .invalid_samples == 0 and .inference_p95_us != null)
  ' "$history_out"

  test -s "$ridge_out"
  test -s "$hdc_out"
  jq -n --slurpfile ridge "$ridge_out" --slurpfile hdc "$hdc_out" --slurpfile mlp "$mlp_out" --slurpfile history "$history_out" '
    ($ridge[0]) as $r
    | ($hdc[0]) as $h
    | ($mlp[0]) as $m
    | ($history[0]) as $w
    | ($r.source_receipt.training_dataset_sha256 == $h.source_receipt.training_dataset_sha256)
    and ($r.source_receipt.training_dataset_sha256 == $m.source_receipt.training_dataset_sha256)
    and ($r.source_receipt.training_dataset_sha256 == $w.source_receipt.training_dataset_sha256)
    and ($r.source_receipt.symthaea_git_sha == $h.source_receipt.symthaea_git_sha)
    and ($r.source_receipt.symthaea_git_sha == $m.source_receipt.symthaea_git_sha)
    and ($r.source_receipt.symthaea_git_sha == $w.source_receipt.symthaea_git_sha)
    and ($r.source_receipt.scenario_matrix_sha256 == $h.source_receipt.scenario_matrix_sha256)
    and ($r.source_receipt.scenario_matrix_sha256 == $m.source_receipt.scenario_matrix_sha256)
    and ($r.source_receipt.scenario_matrix_sha256 == $w.source_receipt.scenario_matrix_sha256)
    and ($r.evaluation.episodes_requested == $h.evaluation.episodes_requested)
    and ($r.evaluation.episodes_requested == $m.evaluation.episodes_requested)
    and ($r.evaluation.episodes_requested == $w.evaluation.episodes_requested)
    and ($r.evaluation.steps_per_episode == $h.evaluation.steps_per_episode)
    and ($r.evaluation.steps_per_episode == $m.evaluation.steps_per_episode)
    and ($r.evaluation.steps_per_episode == $w.evaluation.steps_per_episode)
    and ($r.evaluation.scenario_ids == $h.evaluation.test_scenario_ids)
    and ($r.evaluation.scenario_ids == $m.evaluation.test_scenario_ids)
    and ($r.evaluation.scenario_ids == $w.evaluation.test_scenario_ids)
    and ([$r.evaluation.episode_details[].episode_seed_hex] == $h.evaluation.test_episode_seed_hexes)
    and ([$r.evaluation.episode_details[].episode_seed_hex] == $m.evaluation.test_episode_seed_hexes)
    and ([$r.evaluation.episode_details[].episode_seed_hex] == $w.evaluation.test_episode_seed_hexes)
    and ([$r.evaluation.episode_details[].scenario_id] == $h.evaluation.test_episode_scenario_ids)
    and ([$r.evaluation.episode_details[].scenario_id] == [$m.evaluation.episode_details[].scenario_id])
    and ([$r.evaluation.episode_details[].scenario_id] == [$w.evaluation.episode_details[].scenario_id])
  '

  export STFR_RIDGE_OUTPUT_PATH="$ridge_out"
  export STFR_HDC_OUTPUT_PATH="$hdc_out"
  export STFR_MLP_OUTPUT_PATH="$mlp_out"
  export STFR_HISTORY_MLP_OUTPUT_PATH="$history_out"
  comparison_out="${STFR_COMPARISON_OUTPUT_PATH:-$SYMTROPY_PATH/symtropy-residual-model-paired-comparison.json}"
  cargo run --manifest-path "$integration_manifest" --example paired_residual_model_comparison > "$comparison_out"
  jq -e '
    .comparison_id == "symtropy-residual-model-paired-comparison-v0.1"
    and .claim_ceiling == "paired_episode_level_one_step_prediction_comparison_conditional_on_smoke_scenario_families"
    and (.source_receipt.training_dataset_sha256 | test("^[0-9a-f]{64}$"))
    and .design.bootstrap_unit == "episode"
    and .design.bootstrap_replicates == 10000
    and .design.seed_hex_equality_required == true
    and .design.scenario_order_equality_required == true
    and (.model_summary.raw_state_ridge_mean_episode_rmse_m_s2 >= 0)
    and (.model_summary.compact_mlp_mean_episode_rmse_m_s2 >= 0)
    and (.model_summary.hdc_ltc_reservoir_mean_episode_rmse_m_s2 >= 0)
    and (.model_summary.explicit_history_mlp_mean_episode_rmse_m_s2 >= 0)
    and (.paired_comparisons | length) == 6
    and all(.paired_comparisons[]; (.stratified_episode_bootstrap_95_ci_m_s2 | length) == 2)
    and (.family_summary | length) == 3
    and (.per_episode | length) == .design.episodes
    and all(.per_episode[]; .raw_state_ridge_rmse_m_s2 >= 0 and .compact_mlp_rmse_m_s2 >= 0 and .explicit_history_mlp_rmse_m_s2 >= 0 and .hdc_ltc_reservoir_rmse_m_s2 >= 0)
  ' "$comparison_out"
fi

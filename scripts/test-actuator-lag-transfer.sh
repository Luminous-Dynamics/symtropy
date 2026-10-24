#!/usr/bin/env bash
set -euo pipefail

: "${SYMTROPY_PATH:?SYMTROPY_PATH must point at the Symtropy checkout}"
: "${SYMTHAEA_PATH:?SYMTHAEA_PATH must point at the pinned Symthaea checkout}"
test -f "$SYMTROPY_PATH/scripts/test-real-symthaea-in-symtropy.sh"

# Internal controlled stress experiment only. The dataset is collected with a 20 ms
# first-order command lag, then tested at direct actuation and 10/20/50 ms. Those are
# declared simulator parameters—not calibrated motor constants or real-flight claims.
train_tau_s="${STFR_ACTUATOR_TRAIN_TAU_S:-0.020}"
jq -e --null-input --argjson train_tau "$train_tau_s" '$train_tau > 0' > /dev/null
output_dir="${STFR_ACTUATOR_SWEEP_OUTPUT_DIR:-$SYMTROPY_PATH/actuator-lag-transfer-results}"
mkdir -p "$output_dir"

comparison_files=()
first_training_digest=""
first_run=1

for profile in "0.000:direct" "0.010:10ms" "0.020:20ms" "0.050:50ms"; do
  IFS=: read -r eval_tau_s tag <<< "$profile"
  echo "=== Actuator lag transfer: train=${train_tau_s}s eval=${eval_tau_s}s ($tag) ==="

  dataset_path="$output_dir/training-dataset-train-${train_tau_s}s.json"
  ridge_path="$output_dir/ridge-${tag}.json"
  hdc_path="$output_dir/hdc-ltc-${tag}.json"
  mlp_path="$output_dir/current-mlp-${tag}.json"
  history_path="$output_dir/history-mlp-${tag}.json"
  comparison_path="$output_dir/comparison-${tag}.json"

  skip_tests=0
  if [ "$first_run" -eq 0 ]; then
    # The first invocation runs the Rust unit tests; later invocations reuse Cargo's
    # compilation cache and only rebuild/run the benchmark binaries as needed.
    skip_tests=1
  fi

  STFR_RUN_BENCHMARK=1 \
  STFR_RESIDUAL_ONLY=1 \
  STFR_SKIP_UNIT_TESTS="$skip_tests" \
  STFR_DATASET_ACTUATOR_LAG_TAU_S="$train_tau_s" \
  STFR_EVAL_ACTUATOR_LAG_TAU_S="$eval_tau_s" \
  STFR_DATASET_OUTPUT_PATH="$dataset_path" \
  STFR_RIDGE_OUTPUT_PATH="$ridge_path" \
  STFR_HDC_OUTPUT_PATH="$hdc_path" \
  STFR_MLP_OUTPUT_PATH="$mlp_path" \
  STFR_HISTORY_MLP_OUTPUT_PATH="$history_path" \
  STFR_COMPARISON_OUTPUT_PATH="$comparison_path" \
  SYMTROPY_PATH="$SYMTROPY_PATH" \
  SYMTHAEA_PATH="$SYMTHAEA_PATH" \
    bash "$SYMTROPY_PATH/scripts/test-real-symthaea-in-symtropy.sh"

  test -s "$dataset_path"
  test -s "$ridge_path"
  test -s "$hdc_path"
  test -s "$mlp_path"
  test -s "$history_path"
  test -s "$comparison_path"

  jq -e --argjson train_tau "$train_tau_s" '
    ((.plant_configuration.actuator_lag_time_constant_s // 0) == $train_tau)
    and .plant_configuration.actuator_model == (if $train_tau == 0 then "direct_command_application" else "first_order_exponential_command_filter" end)
  ' "$dataset_path" > /dev/null

  observed_digest="$(jq -r '.source_receipt.training_dataset_sha256' "$comparison_path")"
  if [ -z "$first_training_digest" ]; then
    first_training_digest="$observed_digest"
  elif [ "$observed_digest" != "$first_training_digest" ]; then
    echo "training dataset digest changed across actuator evaluation profiles" >&2
    exit 1
  fi

  jq -e --arg tag "$tag" --argjson eval_tau "$eval_tau_s" --argjson train_tau "$train_tau_s" '
    .comparison_id == "symtropy-residual-model-paired-comparison-v0.1"
    and .plant_configuration.training.actuator_lag_time_constant_s == $train_tau
    and ((.plant_configuration.evaluation.actuator_lag_time_constant_s // 0) == $eval_tau)
    and ((.plant_configuration.training.actuator_lag_time_constant_s // 0) == $train_tau)
    and ((.plant_configuration.evaluation.actuator_lag_time_constant_s // 0) == $eval_tau)
    and .plant_configuration.training.claim_ceiling == "internal_actuator_lag_stress_configuration_not_calibrated_real_flight_model"
    and .plant_configuration.evaluation.claim_ceiling == "internal_actuator_lag_stress_configuration_not_calibrated_real_flight_model"
    and (.source_receipt.training_dataset_sha256 | test("^[0-9a-f]{64}$"))
    and (.paired_comparisons | length) == 10
    and (.per_episode | length) == .design.episodes
    and (.family_summary | length) == 3
    and all(.per_episode[];
      .raw_state_ridge_rmse_m_s2 >= 0
      and .compact_mlp_rmse_m_s2 >= 0
      and .explicit_history_mlp_rmse_m_s2 >= 0
      and .hdc_ltc_reservoir_rmse_m_s2 >= 0
      and .hdc_ltc_reset_each_step_rmse_m_s2 >= 0
    )
  ' "$comparison_path" > /dev/null

  comparison_files+=("$comparison_path")
  case "$tag" in
    direct) comparison_direct="$comparison_path" ;;
    10ms) comparison_10ms="$comparison_path" ;;
    20ms) comparison_20ms="$comparison_path" ;;
    50ms) comparison_50ms="$comparison_path" ;;
    *) echo "unknown actuator profile: $tag" >&2; exit 1 ;;
  esac
  first_run=0
done

summary_path="${STFR_ACTUATOR_SWEEP_SUMMARY_PATH:-$output_dir/actuator-lag-transfer-sweep.json}"
STFR_ACTUATOR_DIRECT_COMPARISON_PATH="$comparison_direct" \
STFR_ACTUATOR_10MS_COMPARISON_PATH="$comparison_10ms" \
STFR_ACTUATOR_20MS_COMPARISON_PATH="$comparison_20ms" \
STFR_ACTUATOR_50MS_COMPARISON_PATH="$comparison_50ms" \
  cargo run --manifest-path "$SYMTROPY_PATH/integration/symthaea-physics-loop/Cargo.toml" \
    --example actuator_lag_transfer_summary > "$summary_path"

jq -e '
  .benchmark_id == "symtropy-actuator-lag-transfer-sweep-v0.1"
  and .claim_ceiling == "internal_simulator_actuator_lag_generalization_prediction_only_not_real_flight_or_control_evidence"
  and .design.all_training_dataset_digests_match == true
  and (.evaluation_profiles | length) == 4
  and ([.evaluation_profiles[].actuator_lag_time_constant_s // 0] == [0, 0.01, 0.02, 0.05])
  and (.resilience_contrasts | length) == 3
  and all(.evaluation_profiles[];
    (.paired_rmse_delta_vs_20ms | length) == 5
    and (.relative_rmse_degradation_vs_20ms | length) == 5
    and all(.paired_rmse_delta_vs_20ms[]; (.stratified_episode_bootstrap_95_ci_m_s2 | length) == 2)
  )
' "$summary_path" > /dev/null

  . as $runs
  | ($runs
      | map(select((.plant_configuration.evaluation.actuator_lag_time_constant_s // 0) == 0.020))
      | .[0]) as $matched
  | {
      benchmark_id: "symtropy-actuator-lag-transfer-sweep-v0.1",
      claim_ceiling: "internal_simulator_actuator_lag_generalization_prediction_only_not_real_flight_or_control_evidence",
      training_actuator_lag_time_constant_s: $training_tau,
      matched_reference_eval_tau_s: 0.020,
      evaluation_profiles: ($runs | map(
        . as $run
        | {
            actuator_lag_time_constant_s: .plant_configuration.evaluation.actuator_lag_time_constant_s,
            training_dataset_sha256: .source_receipt.training_dataset_sha256,
            model_summary: .model_summary,
            state_memory_ablation: .state_memory_ablation,
            paired_comparisons: .paired_comparisons,
            relative_rmse_degradation_vs_20ms: {
              raw_state_ridge: (($run.model_summary.raw_state_ridge_mean_episode_rmse_m_s2 - $matched.model_summary.raw_state_ridge_mean_episode_rmse_m_s2) / ([($matched.model_summary.raw_state_ridge_mean_episode_rmse_m_s2 | fabs), 1.0e-9] | max)),
              compact_mlp: (($run.model_summary.compact_mlp_mean_episode_rmse_m_s2 - $matched.model_summary.compact_mlp_mean_episode_rmse_m_s2) / ([($matched.model_summary.compact_mlp_mean_episode_rmse_m_s2 | fabs), 1.0e-9] | max)),
              explicit_history_mlp: (($run.model_summary.explicit_history_mlp_mean_episode_rmse_m_s2 - $matched.model_summary.explicit_history_mlp_mean_episode_rmse_m_s2) / ([($matched.model_summary.explicit_history_mlp_mean_episode_rmse_m_s2 | fabs), 1.0e-9] | max)),
              persistent_hdc_ltc: (($run.model_summary.hdc_ltc_reservoir_mean_episode_rmse_m_s2 - $matched.model_summary.hdc_ltc_reservoir_mean_episode_rmse_m_s2) / ([($matched.model_summary.hdc_ltc_reservoir_mean_episode_rmse_m_s2 | fabs), 1.0e-9] | max)),
              ltc_reset_each_step_hdc: (($run.model_summary.hdc_ltc_reset_each_step_mean_episode_rmse_m_s2 - $matched.model_summary.hdc_ltc_reset_each_step_mean_episode_rmse_m_s2) / ([($matched.model_summary.hdc_ltc_reset_each_step_mean_episode_rmse_m_s2 | fabs), 1.0e-9] | max))
            }
          }
      )),
      all_training_dataset_digests_match:
        ([$runs[].source_receipt.training_dataset_sha256] | unique | length) == 1,
      interpretation:
        "Compare which representation degrades least as held-out actuator response changes. Fractional deltas are relative to the 20 ms evaluation profile and are not evidence of real-flight dynamics or control performance."
    }
' "${comparison_files[@]}" > "$summary_path"

echo "Actuator lag transfer sweep receipt: $summary_path"

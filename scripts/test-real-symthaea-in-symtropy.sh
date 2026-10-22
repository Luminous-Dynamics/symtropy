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
        and .rmse_m != null
        and .within_5cm_fraction != null
        and .max_position_error_m != null
        and .controller_compute_p95_us != null
      )
    )
  ' "$out"
fi

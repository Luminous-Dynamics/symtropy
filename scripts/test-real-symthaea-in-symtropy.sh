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

cargo test --manifest-path "$integration_manifest" --tests -- --nocapture
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

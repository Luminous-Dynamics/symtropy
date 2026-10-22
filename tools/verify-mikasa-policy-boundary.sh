#!/usr/bin/env bash
set -euo pipefail

# VIS-MEM-017 companion audit.
# Validates the pinned MIKASA runner/wrapper source path only; this is not
# dynamic instrumentation and cannot prove arbitrary policy code is oracle-free.

upstream="${1:-}"
if [[ -z "$upstream" || ! -d "$upstream/.git" ]]; then
  echo "usage: bash tools/verify-mikasa-policy-boundary.sh <pinned-MIKASA-Robo-checkout>" >&2
  exit 2
fi

readonly PIN="509b875f3d207c287497c0a897661062de928bb0"
readonly BENCH="mikasa_robo_suite/vla/benchmarking.py"
readonly APPLY="mikasa_robo_suite/vla/utils/apply_wrappers.py"
readonly WRAPPERS="mikasa_robo_suite/vla/utils/wrappers.py"
readonly PPO_WRAPPER="baselines/ppo/ppo_memtasks.py"
readonly TRACE_TASK="mikasa_robo_suite/vla/memory_envs/trace_shape_seq_vla.py"
readonly ROSTER="tools/vision-memory-mikasa-long-tasks.tsv"

fail() { echo "FAIL: $*" >&2; exit 1; }
require_literal() {
  local file="$1" literal="$2" label="$3"
  grep -Fq -- "$literal" "$file" || fail "missing audited contract: $label"
  printf 'CHECK\t%s\tPASS\n' "$label" >> "$receipt"
}

actual_commit="$(git -C "$upstream" rev-parse HEAD)"
[[ "$actual_commit" == "$PIN" ]] || fail "upstream HEAD $actual_commit != pinned $PIN"

bench="$upstream/$BENCH"
apply="$upstream/$APPLY"
wrappers="$upstream/$WRAPPERS"
ppo_wrapper="$upstream/$PPO_WRAPPER"
trace_task="$upstream/$TRACE_TASK"
for file in "$bench" "$apply" "$wrappers" "$ppo_wrapper" "$trace_task" "$ROSTER"; do
  [[ -f "$file" ]] || fail "missing pinned source file: $file"
done

# Bind this audit to the reviewed Git blob contents, not just a movable branch.
[[ "$(git -C "$upstream" rev-parse "$PIN:$BENCH")" == "28bcb49e10ef87735ec02281f6d19216a340ce3d" ]] || fail "benchmarking.py blob changed"
[[ "$(git -C "$upstream" rev-parse "$PIN:$APPLY")" == "5cf8e6d82b9cf5242d22b8402ad75cdb9ca142b1" ]] || fail "apply_wrappers.py blob changed"
[[ "$(git -C "$upstream" rev-parse "$PIN:$WRAPPERS")" == "829b65903ac56747868d8e187e2c4249b1c2800e" ]] || fail "wrappers.py blob changed"
[[ "$(git -C "$upstream" rev-parse "$PIN:$PPO_WRAPPER")" == "99af61c1c4ade67a152b90863f517474e0e940e9" ]] || fail "FlattenRGBDObservationWrapper source blob changed"
[[ "$(git -C "$upstream" rev-parse "$PIN:$TRACE_TASK")" == "115b62efa6e8f53114e4dec347da9317463f84ae" ]] || fail "TraceShapeSeq task source blob changed"

receipt="mikasa-policy-boundary-receipt.tsv"
{
  printf 'META\tstatus\tstatic_source_audit_only\n'
  printf 'META\tnative_benchmark_admission\tBLOCKED_PENDING_TASK_CUE_INPUT_BOUNDARY_REVIEW\n'
  printf 'META\tbenchmark_source_commit\t%s\n' "$actual_commit"
  printf 'META\tbenchmarking_py_blob\t%s\n' "$(git -C "$upstream" rev-parse "$PIN:$BENCH")"
  printf 'META\tapply_wrappers_py_blob\t%s\n' "$(git -C "$upstream" rev-parse "$PIN:$APPLY")"
  printf 'META\tvla_wrappers_py_blob\t%s\n' "$(git -C "$upstream" rev-parse "$PIN:$WRAPPERS")"
  printf 'META\tppo_memtasks_py_blob\t%s\n' "$(git -C "$upstream" rev-parse "$PIN:$PPO_WRAPPER")"
  printf 'META\ttrace_shape_seq_py_blob\t%s\n' "$(git -C "$upstream" rev-parse "$PIN:$TRACE_TASK")"
} > "$receipt"

# The runner contract accepts a single observation mapping, builds obs via the
# wrapped env, and passes only that object to forward(); info and env are not
# arguments to forward. A source change requires re-review and a new pin.
require_literal "$bench" 'def forward(self, obs: Mapping[str, Any])' 'policy_interface_observation_only'
require_literal "$bench" 'obs, _ = env.reset(seed=episode_seed)' 'episode_observation_from_reset'
require_literal "$bench" 'policy.forward(obs)' 'runner_passes_observation_only'
require_literal "$bench" 'WRAPPER_CHAIN = "apply_mikasa_vla_wrappers(include_overlays=False)"' 'canonical_wrapper_metadata'
require_literal "$bench" 'return apply_mikasa_vla_wrappers(env, include_overlays=include_overlays)' 'runner_uses_vla_wrapper_stack'
require_literal "$apply" 'FlattenRGBDObservationWrapper(env, rgb=True, depth=False, state=False, oracle=False, joints=True)' 'wrapper_requests_oracle_field_exclusion'
require_literal "$wrappers" 'obs.update({"task_cue": task_cue_, "oracle_info": oracle_info_})' 'upstream_privileged_fields_identified_pre_filter'
require_literal "$ppo_wrapper" 'ret["task_cue"] = observation["task_cue"]' 'flatten_wrapper_copies_task_cue_before_sentinel_filter'
require_literal "$ppo_wrapper" 'if "task_cue" in ret.keys() and (ret["task_cue"] == 4242424242).any().item():' 'task_cue_removed_only_when_sentinel'
require_literal "$trace_task" 'self.task_cue = self.shape_sequence.to(torch.int64)' 'trace_shape_seq_sets_task_cue_to_target_sequence'
require_literal "$ROSTER" 'TraceShapeSeq' 'long_roster_includes_trace_shape_seq'

if grep -Eq 'policy\.forward\([^)]*,|policy\.forward\(env|policy\.forward\([^)]*info' "$bench"; then
  fail "policy forward call appears to receive additional runner objects"
fi
printf 'CHECK\tno_extra_forward_arguments\tPASS\n' >> "$receipt"
if grep -Fq 'TraceShapeSeq' "$ROSTER"; then
  printf 'CHECK\tnon_rotate_task_cue_path\tBLOCKER\n' >> "$receipt"
  printf 'META\tblocker\tTraceShapeSeq is in the Long roster; task_cue copies shape_sequence and the pinned wrapper only removes the sentinel, so the cue can reach policy.forward(obs)\n' >> "$receipt"
  printf 'META\tmitigation\tstrip task_cue at the policy adapter boundary or fix/pin upstream wrapper before reporting canonical Long results\n' >> "$receipt"
fi

# Digest sources and receipt so later reviews can bind their conclusions to
# exact, inspectable content. This remains a static source audit.
sha256sum "$bench" "$apply" "$wrappers" "$ppo_wrapper" "$trace_task" "$ROSTER" "$receipt" > mikasa-policy-boundary.sha256
sha256sum -c mikasa-policy-boundary.sha256
cat "$receipt"
echo "BLOCKER: native benchmark admission is blocked until the TraceShapeSeq task_cue path is stripped or fixed and the source pin is re-audited."

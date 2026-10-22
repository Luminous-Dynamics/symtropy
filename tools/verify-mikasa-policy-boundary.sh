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
for file in "$bench" "$apply" "$wrappers"; do
  [[ -f "$file" ]] || fail "missing pinned source file: $file"
done

# Bind this audit to the reviewed Git blob contents, not just a movable branch.
[[ "$(git -C "$upstream" rev-parse "$PIN:$BENCH")" == "28bcb49e10ef87735ec02281f6d19216a340ce3d" ]] || fail "benchmarking.py blob changed"
[[ "$(git -C "$upstream" rev-parse "$PIN:$APPLY")" == "5cf8e6d82b9cf5242d22b8402ad75cdb9ca142b1" ]] || fail "apply_wrappers.py blob changed"
[[ "$(git -C "$upstream" rev-parse "$PIN:$WRAPPERS")" == "829b65903ac56747868d8e187e2c4249b1c2800e" ]] || fail "wrappers.py blob changed"

receipt="mikasa-policy-boundary-receipt.tsv"
{
  printf 'META\tstatus\tsource_boundary_static_contract_validated_not_runtime_isolation_proven\n'
  printf 'META\tbenchmark_source_commit\t%s\n' "$actual_commit"
  printf 'META\tbenchmarking_py_blob\t%s\n' "$(git -C "$upstream" rev-parse "$PIN:$BENCH")"
  printf 'META\tapply_wrappers_py_blob\t%s\n' "$(git -C "$upstream" rev-parse "$PIN:$APPLY")"
  printf 'META\tvla_wrappers_py_blob\t%s\n' "$(git -C "$upstream" rev-parse "$PIN:$WRAPPERS")"
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

if grep -Eq 'policy\.forward\([^)]*,|policy\.forward\(env|policy\.forward\([^)]*info' "$bench"; then
  fail "policy forward call appears to receive additional runner objects"
fi
printf 'CHECK\tno_extra_forward_arguments\tPASS\n' >> "$receipt"

# Digest sources and receipt so later reviews can bind their conclusions to
# exact, inspectable content. This remains a static source audit.
sha256sum "$bench" "$apply" "$wrappers" "$receipt" > mikasa-policy-boundary.sha256
sha256sum -c mikasa-policy-boundary.sha256
cat "$receipt"
echo "NOTE: static source contract only; policy internals and runtime dataflow still require a separate audit."

#!/usr/bin/env bash
set -euo pipefail

# Validate a complete official MIKASA-Robo-VLA Long result directory.
# This checks artifact integrity, not policy correctness or SOTA performance.
# Usage: verify-mikasa-long-results.sh RUN_DIR RECEIPT_DIR
# Dependency: bash, jq, awk, sort, diff, sha256sum.

if [[ "$#" -ne 2 ]]; then
  echo "usage: $0 RUN_DIR RECEIPT_DIR" >&2
  exit 64
fi

RUN_DIR=$1
RECEIPT_DIR=$2
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
MANIFEST="$SCRIPT_DIR/vision-memory-mikasa-long-tasks.tsv"
PINNED_COMMIT="509b875f3d207c287497c0a897661062de928bb0"
MANIFEST_BLOB_SHA1="613c571bc4a5e1585c995249adb31d594601822e"
LOCAL_ROSTER_BLOB_SHA1="61fc51de3075e6a76ab87881aabd17a6f8ab5256"
START_SEED=4242424242
EPISODES=50
EXPECTED_TASKS=22

fail() {
  echo "MIKASA_VERIFY_FAIL: $*" >&2
  exit 1
}

close_enough() {
  awk -v a="$1" -v b="$2" -v tolerance="0.000001" 'BEGIN {
    difference = a - b
    if (difference < 0) difference = -difference
    exit !(difference <= tolerance)
  }'
}

[[ -d "$RUN_DIR" ]] || fail "run directory does not exist"
[[ -f "$RUN_DIR/summary.json" ]] || fail "summary.json is missing"
[[ ! -L "$RUN_DIR/summary.json" ]] || fail "summary.json must not be a symlink"
[[ -f "$MANIFEST" ]] || fail "pinned Long task manifest is missing"
[[ ! -e "$RECEIPT_DIR" ]] || fail "receipt directory already exists; use a fresh output path"
command -v jq >/dev/null || fail "jq is required"
command -v sha256sum >/dev/null || fail "sha256sum is required"
command -v git >/dev/null || fail "git is required to verify the pinned roster"

[[ "$(git hash-object "$MANIFEST")" == "$LOCAL_ROSTER_BLOB_SHA1" ]] || fail "checked-in Long roster differs from pinned validated roster"
VERIFY_SCRIPT_SHA256=$(sha256sum "$0" | awk '{print $1}')

TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

# Fail closed on malformed rows and duplicate environment identifiers.
awk -F '\t' '!/^#/ && NF == 4 { if (seen[$2]++) exit 3; count++ } END { if (count != 22) exit 1 }' "$MANIFEST"   || fail "manifest must have 22 unique four-column task rows"

TASKS_JSON=$(awk -F '\t' '!/^#/ && NF == 4 { print $2 }' "$MANIFEST" | jq -R -s 'split("\n") | map(select(length > 0))')
TYPES_JSON=$(awk -F '\t' '!/^#/ && NF == 4 { print $3 }' "$MANIFEST" | sort -u | jq -R -s 'split("\n") | map(select(length > 0))')

jq -e --argjson tasks "$TASKS_JSON" --argjson types "$TYPES_JSON" '
  .split == "Long"
  and (.tasks | type == "array")
  and (.tasks | length == 22)
  and ((.tasks | unique | length) == 22)
  and ((.tasks | sort) == ($tasks | sort))
  and (.per_task_sr | type == "object")
  and ((.per_task_sr | keys | sort) == ($tasks | sort))
  and (.per_task_mean_return | type == "object")
  and ((.per_task_mean_return | keys | sort) == ($tasks | sort))
  and (.sr_per_memory_type | type == "object")
  and ((.sr_per_memory_type | keys | sort) == ($types | sort))
  and (.sr_split | type == "number" and . >= 0 and . <= 1)
' "$RUN_DIR/summary.json" >/dev/null   || fail "summary schema, split, task roster, or aggregate key sets are invalid"

json_count=$(find "$RUN_DIR" -maxdepth 1 -type f -name '*.json' ! -name 'summary.json' | wc -l | tr -d ' ')
[[ "$json_count" == "$EXPECTED_TASKS" ]] || fail "expected exactly 22 per-task JSONs; found $json_count"
extra_or_missing=$(comm -3 \
  <(awk -F '\t' '!/^#/ && NF == 4 { print $2 }' "$MANIFEST" | sort) \
  <(find "$RUN_DIR" -maxdepth 1 -type f -name '*.json' ! -name 'summary.json' -printf '%f\n' | sed 's/\.json$//' | sort))
[[ -z "$extra_or_missing" ]] || fail "result filenames do not exactly match pinned Long roster: $extra_or_missing"

BASE_MODEL=""
BASE_MODEL_SHA=""
BASE_CHUNK=""
: > "$TMP_DIR/task-scores.tsv"

while IFS=$'\t' read -r task_id task memory_type horizon; do
  [[ -z "$task_id" || "$task_id" == \#* ]] && continue
  file="$RUN_DIR/$task.json"
  [[ -f "$file" ]] || fail "missing task JSON: $task.json"
  [[ ! -L "$file" ]] || fail "task JSON must not be a symlink: $task.json"

  jq -e --arg task "$task" --arg memory_type "$memory_type" \
    --argjson start "$START_SEED" --argjson episodes "$EPISODES" \
    --arg commit "$PINNED_COMMIT" '
      .env_id == $task
      and .split == "Long"
      and .memory_type == $memory_type
      and .start_seed == $start
      and .n_episodes == $episodes
      and .benchmark_commit == $commit
      and .obs_mode == "rgb"
      and .control_mode == "pd_ee_delta_pose"
      and .wrapper_chain == "apply_mikasa_vla_wrappers(include_overlays=False)"
      and (.successes | type == "array" and length == $episodes and all(.[]; type == "boolean"))
      and (.returns | type == "array" and length == $episodes and all(.[]; type == "number"))
      and (.episode_lengths | type == "array" and length == $episodes and all(.[]; type == "number" and . >= 0))
      and (.episode_seeds == [range($start; $start + $episodes)])
      and (.sr | type == "number" and . >= 0 and . <= 1)
      and ((.sr - (([.successes[] | if . then 1 else 0 end] | add) / $episodes) | fabs) < 0.000001)
      and (.mean_return | type == "number")
      and ((.mean_return - (([.returns[]] | add) / $episodes) | fabs) < 0.000001)
      and (.action_chunk_size | type == "number" and . > 0)
      and (.model.name | type == "string" and length > 0)
      and (.model.config | type == "object")
      and (([.model.config | .. | objects | select(has("oracle_info") or has("task_cue"))] | length) == 0)
    ' "$file" >/dev/null || fail "noncanonical or malformed task artifact: $task.json"

  model_json=$(jq -cS '.model' "$file")
  model_sha=$(printf '%s' "$model_json" | sha256sum | awk '{print $1}')
  chunk=$(jq -r '.action_chunk_size' "$file")
  if [[ -z "$BASE_MODEL" ]]; then
    BASE_MODEL=$model_json
    BASE_MODEL_SHA=$model_sha
    BASE_CHUNK=$chunk
  else
    [[ "$model_json" == "$BASE_MODEL" ]] || fail "model/model.config changes between task outputs at $task"
    [[ "$chunk" == "$BASE_CHUNK" ]] || fail "action_chunk_size changes between task outputs at $task"
  fi

  sr=$(jq -r '.sr' "$file")
  mean_return=$(jq -r '.mean_return' "$file")
  summary_sr=$(jq -r --arg task "$task" '.per_task_sr[$task]' "$RUN_DIR/summary.json")
  summary_return=$(jq -r --arg task "$task" '.per_task_mean_return[$task]' "$RUN_DIR/summary.json")
  close_enough "$sr" "$summary_sr" || fail "summary per-task SR mismatch for $task"
  close_enough "$mean_return" "$summary_return" || fail "summary per-task mean-return mismatch for $task"
  printf '%s\t%s\t%.9f\t%.9f\n' "$task" "$memory_type" "$sr" "$mean_return" >> "$TMP_DIR/task-scores.tsv"
done < "$MANIFEST"

CALCULATED_SR=$(awk -F '\t' '{sum += $3; count++} END {if (count == 0) exit 1; printf "%.9f\n", sum / count}' "$TMP_DIR/task-scores.tsv")
SUMMARY_SR=$(jq -r '.sr_split' "$RUN_DIR/summary.json")
close_enough "$CALCULATED_SR" "$SUMMARY_SR" || fail "summary sr_split $SUMMARY_SR disagrees with canonical task mean $CALCULATED_SR"

while IFS=$'\t' read -r memory_type calculated; do
  actual=$(jq -r --arg type "$memory_type" '.sr_per_memory_type[$type]' "$RUN_DIR/summary.json")
  close_enough "$calculated" "$actual" || fail "memory-type SR mismatch for $memory_type"
done < <(awk -F '\t' '{sum[$2] += $3; count[$2]++} END {for (type in sum) printf "%s\t%.9f\n", type, sum[type] / count[type]}' "$TMP_DIR/task-scores.tsv" | sort)

mkdir -p "$RECEIPT_DIR"
{
  printf 'META\tstatus\tcontract_validated_not_performance_evaluated\n'
  printf 'META\tbenchmark\tMIKASA-Robo-VLA Long\n'
  printf 'META\tbenchmark_commit\t%s\n' "$PINNED_COMMIT"
  printf 'META\tmanifest_upstream_git_blob_sha1\t%s\n' "$MANIFEST_BLOB_SHA1"
  printf 'META\tlocal_roster_git_blob_sha1\t%s\n' "$LOCAL_ROSTER_BLOB_SHA1"
  printf 'META\tverifier_script_sha256\t%s\n' "$VERIFY_SCRIPT_SHA256"
  printf 'META\tmanifest_tasks\t%d\n' "$EXPECTED_TASKS"
  printf 'META\tepisodes_per_task\t%d\n' "$EPISODES"
  printf 'META\tseed_start\t%d\n' "$START_SEED"
  printf 'META\tseed_end\t%d\n' "$((START_SEED + EPISODES - 1))"
  printf 'META\tmodel_config_sha256\t%s\n' "$BASE_MODEL_SHA"
  printf 'META\taction_chunk_size\t%s\n' "$BASE_CHUNK"
  printf 'META\tmacro_task_success_rate\t%s\n' "$CALCULATED_SR"
  printf 'META\tverification_scope\tartifact integrity only; source-level oracle-access audit required separately\n'
  printf 'TASK_HEADER\tenv_id\tmemory_type\tsuccess_rate\tmean_return\n'
  cat "$TMP_DIR/task-scores.tsv" | awk -F '\t' 'BEGIN {OFS="\t"} {printf "TASK\t%s\t%s\t%.9f\t%.9f\n", $1,$2,$3,$4}'
} > "$RECEIPT_DIR/scorecard.tsv"

{
  while IFS=$'\t' read -r task_id task memory_type horizon; do
    [[ -z "$task_id" || "$task_id" == \#* ]] && continue
    digest=$(sha256sum "$RUN_DIR/$task.json" | awk '{print $1}')
    printf '%s\t%s\n' "$digest" "$task.json"
  done < "$MANIFEST"
  summary_digest=$(sha256sum "$RUN_DIR/summary.json" | awk '{print $1}')
  printf '%s\t%s\n' "$summary_digest" "summary.json"
} > "$RECEIPT_DIR/source-json-sha256.tsv"

(cd "$RECEIPT_DIR" && sha256sum scorecard.tsv source-json-sha256.tsv > receipt.sha256)
echo "MIKASA_VERIFY_PASS: contract_validated_not_performance_evaluated"
echo "Scorecard: $RECEIPT_DIR/scorecard.tsv"
echo "Receipt: $RECEIPT_DIR/receipt.sha256"

#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
VERIFIER="$SCRIPT_DIR/verify-mikasa-long-results.sh"
MANIFEST="$SCRIPT_DIR/vision-memory-mikasa-long-tasks.tsv"
PINNED_COMMIT="509b875f3d207c287497c0a897661062de928bb0"
START_SEED=4242424242
EPISODES=50

command -v jq >/dev/null || { echo "jq is required" >&2; exit 1; }
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

TASKS_JSON=$(awk -F '\t' '!/^#/ && NF == 4 { print $2 }' "$MANIFEST" | jq -R -s 'split("\n") | map(select(length > 0))')
TYPES_JSON=$(awk -F '\t' '!/^#/ && NF == 4 { print $3 }' "$MANIFEST" | sort -u | jq -R -s 'split("\n") | map(select(length > 0))')
SEEDS_JSON=$(jq -n --argjson start "$START_SEED" --argjson n "$EPISODES" '[range($start; $start + $n)]')
SUCCESS_JSON=$(jq -n --argjson n "$EPISODES" '[range(0; $n) | false]')
RETURNS_JSON=$(jq -n --argjson n "$EPISODES" '[range(0; $n) | 0]')
LENGTHS_JSON=$(jq -n --argjson n "$EPISODES" '[range(0; $n) | 1]')

mkdir -p "$TMP_DIR/good"
while IFS=$'\t' read -r task_id task memory_type horizon; do
  [[ -z "$task_id" || "$task_id" == \#* ]] && continue
  jq -n \
    --arg task "$task" --arg memory_type "$memory_type" --arg commit "$PINNED_COMMIT" \
    --argjson start "$START_SEED" --argjson n "$EPISODES" \
    --argjson seeds "$SEEDS_JSON" --argjson successes "$SUCCESS_JSON" \
    --argjson returns "$RETURNS_JSON" --argjson lengths "$LENGTHS_JSON" '
    {
      env_id: $task,
      split: "Long",
      memory_type: $memory_type,
      start_seed: $start,
      n_episodes: $n,
      successes: $successes,
      returns: $returns,
      sr: 0,
      mean_return: 0,
      benchmark_commit: $commit,
      control_mode: "pd_ee_delta_pose",
      obs_mode: "rgb",
      wrapper_chain: "apply_mikasa_vla_wrappers(include_overlays=False)",
      action_chunk_size: 8,
      model: {name: "synthetic-fixture-not-a-robot-policy", config: {fixture: true}},
      episode_lengths: $lengths,
      episode_seeds: $seeds
    }' > "$TMP_DIR/good/$task.json"
done < "$MANIFEST"

jq -n --argjson tasks "$TASKS_JSON" --argjson types "$TYPES_JSON" '
  {
    split: "Long",
    sr_split: 0,
    sr_per_memory_type: (reduce $types[] as $t ({}; .[$t] = 0)),
    tasks: $tasks,
    per_task_sr: (reduce $tasks[] as $t ({}; .[$t] = 0)),
    per_task_mean_return: (reduce $tasks[] as $t ({}; .[$t] = 0))
  }' > "$TMP_DIR/good/summary.json"

bash "$VERIFIER" "$TMP_DIR/good" "$TMP_DIR/receipt-good" > "$TMP_DIR/good.stdout"
grep -Fq 'MIKASA_VERIFY_PASS: contract_validated_not_performance_evaluated' "$TMP_DIR/good.stdout"   || { cat "$TMP_DIR/good.stdout" >&2; exit 1; }
bash "$VERIFIER" "$TMP_DIR/good" "$TMP_DIR/receipt-repeat" > "$TMP_DIR/repeat.stdout"
cmp "$TMP_DIR/receipt-good/scorecard.tsv" "$TMP_DIR/receipt-repeat/scorecard.tsv"
cmp "$TMP_DIR/receipt-good/source-json-sha256.tsv" "$TMP_DIR/receipt-repeat/source-json-sha256.tsv"
cmp "$TMP_DIR/receipt-good/receipt.sha256" "$TMP_DIR/receipt-repeat/receipt.sha256"

expect_failure() {
  local name=$1
  local directory=$2
  if bash "$VERIFIER" "$directory" "$TMP_DIR/receipt-$name" > "$TMP_DIR/$name.stdout" 2>&1; then
    echo "expected verifier rejection for $name but it passed" >&2
    exit 1
  fi
  grep -Fq 'MIKASA_VERIFY_FAIL:' "$TMP_DIR/$name.stdout"     || { cat "$TMP_DIR/$name.stdout" >&2; exit 1; }
}

cp -a "$TMP_DIR/good" "$TMP_DIR/missing"
rm "$TMP_DIR/missing/BunchOfColors3-Long-VLA-v0.json"
expect_failure missing "$TMP_DIR/missing"

cp -a "$TMP_DIR/good" "$TMP_DIR/wrong-seed"
jq '.episode_seeds[0] = 4242424243' "$TMP_DIR/wrong-seed/BatteriesCheckerEasy-6-VLA-v0.json" > "$TMP_DIR/wrong-seed/changed.json"
mv "$TMP_DIR/wrong-seed/changed.json" "$TMP_DIR/wrong-seed/BatteriesCheckerEasy-6-VLA-v0.json"
expect_failure wrong-seed "$TMP_DIR/wrong-seed"

cp -a "$TMP_DIR/good" "$TMP_DIR/wrong-score"
jq '.sr = 0.5' "$TMP_DIR/wrong-score/BatteriesCheckerEasy-6-VLA-v0.json" > "$TMP_DIR/wrong-score/changed.json"
mv "$TMP_DIR/wrong-score/changed.json" "$TMP_DIR/wrong-score/BatteriesCheckerEasy-6-VLA-v0.json"
expect_failure wrong-score "$TMP_DIR/wrong-score"

cp -a "$TMP_DIR/good" "$TMP_DIR/oracle"
jq '.model.config.task_cue = true' "$TMP_DIR/oracle/BatteriesCheckerEasy-6-VLA-v0.json" > "$TMP_DIR/oracle/changed.json"
mv "$TMP_DIR/oracle/changed.json" "$TMP_DIR/oracle/BatteriesCheckerEasy-6-VLA-v0.json"
expect_failure oracle "$TMP_DIR/oracle"

echo "MIKASA_VERIFIER_TEST_PASS: valid fixture accepted; missing task, wrong seed, score mismatch, and oracle config rejected; receipts deterministic"

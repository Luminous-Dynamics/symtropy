#!/usr/bin/env bash
set -euo pipefail

reference_root="${1:?usage: audit_miss_mdp_ppo_reference.sh /path/to/missingness-mdps}"
expected_commit="186c454c8e1581adbbf3cbb49ed9e0df510356f4"
actual_commit="$(git -C "$reference_root" rev-parse HEAD)"
if [[ "$actual_commit" != "$expected_commit" ]]; then
  printf 'wrong upstream commit: expected %s, got %s\n' "$expected_commit" "$actual_commit" >&2
  exit 1
fi

mapfile -t paths < <(
  git -C "$reference_root" ls-tree -r --name-only "$expected_commit" -- experiments/logs_PPO experiments/logs_PPO_stacked     | grep -E '^experiments/logs_PPO(_stacked)?/.+/PPO_[0-9]{3}/results\.json$'     | sort
)

if [[ "${#paths[@]}" -ne 242 ]]; then
  printf 'expected 242 PPO result artifacts, found %s\n' "${#paths[@]}" >&2
  exit 1
fi

for path in "${paths[@]}"; do
  group="${path%/PPO_*/results.json}"
  run_dir="$(basename "$(dirname "$path")")"
  expected_id="${run_dir#PPO_}"
  pattern="${group##*/}"
  if [[ "$group" == experiments/logs_PPO_stacked/* ]]; then
    stacked="yes"
  else
    stacked="no"
  fi
  if [[ "$group" == */icu/* ]]; then
    model="icu"
  elif [[ "$stacked" == "yes" ]]; then
    model="pred"
  else
    model="predator"
  fi
  file="$reference_root/$path"

  jq -e --arg id "$expected_id" --arg group "$group" --arg stacked "$stacked" --arg model "$model" --arg pattern "$pattern" '
    . as $r
    | ($r.id == ($id | tonumber))
    and ($r.algorithm == "PPO")
    and ($r.is_stacked == $stacked)
    and ($r.model == $model)
    and ($r.pattern == $pattern)
    and ($r.train_steps == 1000000)
    and ($r.eval_episodes == 2000)
    and ($r.return_mean | type == "number" and isfinite)
    and ($r.return_std | type == "number" and isfinite and . >= 0)
    and ($r.seed | type == "number" and . > 0)
    and ($r.n_frames | type == "number" and . > 0)
    and ($r.n_layers | type == "number" and . > 0)
    and ($r.n_hidden | type == "number" and . > 0)
    and ($r.ppo_hparams | type == "object" and has("λe") and has("ϵ") and has("a_opt") and has("c_opt") and has("target_kl"))
    and ($r.ppo_hparams["λe"] | type == "number" and isfinite and . >= 0)
    and ($r.ppo_hparams["ϵ"] | type == "number" and isfinite and . > 0)
    and ($r.ppo_hparams.target_kl | type == "number" and isfinite and . > 0)
    and ($r.ppo_hparams.a_opt | type == "object" and has("epochs") and has("lr") and has("batch_size"))
    and ($r.ppo_hparams.c_opt | type == "object" and has("epochs") and has("lr") and has("batch_size"))
  ' "$file" >/dev/null || {
    printf 'invalid PPO result manifest: %s\n' "$path" >&2
    exit 1
  }

  jq -r --arg group "$group" '
    [
      "run",
      $group,
      .is_stacked,
      .model,
      .pattern,
      (.id | tostring),
      (.seed | tostring),
      (.train_steps | tostring),
      (.eval_episodes | tostring),
      (.n_frames | tostring),
      (.n_layers | tostring),
      (.n_hidden | tostring),
      (.return_mean | tostring),
      (.return_std | tostring),
      (.ppo_hparams | tojson)
    ] | @tsv
  ' "$file"
done

#!/usr/bin/env bash
set -euo pipefail

reference_root="${1:?usage: audit_miss_mdp_ppo_reference.sh /path/to/missingness-mdps}"
expected_commit="186c454c8e1581adbbf3cbb49ed9e0df510356f4"
actual_commit="$(git -C "$reference_root" rev-parse HEAD)"
if [[ "$actual_commit" != "$expected_commit" ]]; then
  printf 'wrong upstream commit: expected %s, got %s\n' "$expected_commit" "$actual_commit" >&2
  exit 1
fi

check_blob() {
  local path="$1"
  local expected="$2"
  local actual
  actual="$(git -C "$reference_root" rev-parse "$expected_commit:$path")"
  if [[ "$actual" != "$expected" ]]; then
    printf 'source blob mismatch for %s: expected %s, got %s\n' "$path" "$expected" "$actual" >&2
    exit 1
  fi
}

check_blob "experiments/PPO_baseline.jl" "90b6ad874df4914ed9b66c3be03cbcbed3068c1c"
check_blob "src/PPO.jl" "5486ac1891d9501524a640f060c42c10f8fff04d"
check_blob "src/FrameStacking.jl" "58fa3eb8e95ff093261de09cc843634dc4e9d236"
check_blob "src/LogUtils.jl" "2babf17a1f6c28ed5182a046a78baf292d411d94"
check_blob "src/Missingness.jl" "ab3037fa2d86f696f7af659ad9a43c59b481ae4e"
check_blob "jenv/Project.toml" "a70a44625b91af2d092902efe914b6c420580f08"

grep -Fq 'RUNS = 0  # number of searches' "$reference_root/experiments/PPO_baseline.jl"
grep -Fq 'N_FRAMES = 1' "$reference_root/experiments/PPO_baseline.jl"
grep -Fq 'SEED = 1500' "$reference_root/experiments/PPO_baseline.jl"
grep -Fq 'function train_PPO_agent(' "$reference_root/src/PPO.jl"
grep -Fq 'function sample_ppo_hparams(seed::Integer)' "$reference_root/src/PPO.jl"

if git -C "$reference_root" ls-tree -r --name-only "$expected_commit" -- experiments \
  | grep -Ei '(^|/)[^/]*(PPO.*stack|stack.*PPO)[^/]*\.jl$'; then
  printf 'unexpected dedicated stacked-PPO experiment entrypoint appeared in pinned source tree\n' >&2
  exit 1
fi

mapfile -t paths < <(
  git -C "$reference_root" ls-tree -r --name-only "$expected_commit" -- experiments/logs_PPO experiments/logs_PPO_stacked \
    | grep -E '^experiments/logs_PPO(_stacked)?/.+/PPO_[0-9]{3}/results\.json$' \
    | sort
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
  jq -e \
    --arg id "$expected_id" \
    --arg stacked "$stacked" \
    --arg model "$model" \
    --arg pattern "$pattern" \
    '
      . as $r
      | ($r.id == ($id | tonumber))
      and ($r.algorithm == "PPO")
      and ($r.is_stacked == $stacked)
      and (($r.model == $model) or ($stacked == "yes" and $model == "pred" and $r.model == "predator"))
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
      and ($r.ppo_hparams.a_opt.epochs | type == "number" and isfinite and . > 0)
      and ($r.ppo_hparams.a_opt.lr | type == "number" and isfinite and . > 0)
      and ($r.ppo_hparams.a_opt.batch_size | type == "number" and . > 0)
      and ($r.ppo_hparams.c_opt.epochs | type == "number" and isfinite and . > 0)
      and ($r.ppo_hparams.c_opt.lr | type == "number" and isfinite and . > 0)
      and ($r.ppo_hparams.c_opt.batch_size | type == "number" and . > 0)
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

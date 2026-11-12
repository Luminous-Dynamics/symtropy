#!/usr/bin/env bash
set -euo pipefail

reference_root="${1:?usage: audit_reference.sh /path/to/missingness-mdps}"
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
    printf 'blob mismatch for %s: expected %s, got %s\n' "$path" "$expected" "$actual" >&2
    exit 1
  fi
}

check_blob "LICENSE" "859c41b3529fe6408802113bf8931575339e3262"
check_blob "src/Experiment.jl" "8765725aa592ea5ce3cd628cac6d0b3e52d0abdf"
check_blob "src/Learning.jl" "379f57bb6eca65b3f0bb7dea389c10d4c3ea506b"
check_blob "src/Metrics.jl" "3a96b9184969720fc78f993862e81c5d8b3134ad"
check_blob "src/benchmarks/Icu.jl" "b2ca11258ed56f8abae7e15247b9af2536a246e6"
check_blob "src/benchmarks/PredatorFactored.jl" "fd4fbedc1bf519d04da9eed99bba8538e052cce7"

paths=(
  "experiments/logs/icu/smar/results.json"
  "experiments/logs/icu/mnar-ident/results.json"
  "experiments/logs/icu/mnar-unident/results.json"
  "experiments/logs/predator/smar/results.json"
  "experiments/logs/predator/mnar-unident/results.json"
)
blobs=(
  "33c6bfc9d14f2fd6e174aff758f02f39e581817d"
  "9d6ca66d366bc4c8360f124c7124d320be3a0b43"
  "2bf5bca7ecfa79fe7ef619123e0b7c24186decf7"
  "619033929aed329d75f6800e45fc22e9e2881ece"
  "6bdc39a53cf37916b644478c6ed67af127e2f56e"
)

for index in "${!paths[@]}"; do
  path="${paths[$index]}"
  check_blob "$path" "${blobs[$index]}"
  file="$reference_root/$path"

  jq -er '
    [
      "contract",
      .id,
      (if (.id | startswith("icu/")) then "icu" else "predator" end),
      (if (.id | endswith("/smar")) then "simple_mar"
       elif (.id | endswith("/mnar-ident")) then "identifiable_mnar"
       else "unidentifiable_mnar" end),
      (.pomdp_num_states | tostring),
      (.repetitions | tostring),
      (.dataset_sizes | length | tostring),
      (.miss_classes | length | tostring),
      (.results.learning | length | tostring),
      ([.results.learning[] | select(.finished == true)] | length | tostring),
      ([.results.learning[] | select(has("exception"))] | length | tostring),
      ([.results.learning[] | select(has("exception")) | .exception] | unique | join(";")),
      ([.results.learning[] | select((.message // "") | contains("Failed discrete belief update: new probabilities sum to zero."))] | length | tostring),
      (.results.baseline | length | tostring),
      ([.results.baseline[] | select(.finished != true)] | length | tostring),
      (.results.mconst | length | tostring),
      ([.results.mconst[] | select(.finished != true)] | length | tostring),
      (.results.pomcp | length | tostring),
      ([.results.pomcp[] | select(.finished != true)] | length | tostring),
      (.results.baseline | map(.baseline) | join(";")),
      (.results.learning | map(.algorithm) | unique | join(";")),
      ([.results.baseline[] | select(.baseline == "upper") | .mean][0] | tostring),
      ([.results.baseline[] | select(.baseline == "random") | .mean][0] | tostring)
    ] | @tsv
  ' "$file"

  jq -r '
    .id as $experiment
    | .results.learning[]
    | [
        "run",
        $experiment,
        (.num_samples | tostring),
        (.seed | tostring),
        .miss_class,
        .algorithm,
        (.finished | tostring),
        (has("exception") | tostring),
        (has("mean") | tostring),
        (has("average_total_variation") | tostring),
        (has("worst_total_variation") | tostring),
        (.average_total_variation // "" | tostring),
        (.worst_total_variation // "" | tostring),
        (.mean // "" | tostring),
        (.exception // "" | tostring),
        ((.message // "" | contains("Failed discrete belief update: new probabilities sum to zero.")) | tostring)
      ]
    | @tsv
  ' "$file"
done

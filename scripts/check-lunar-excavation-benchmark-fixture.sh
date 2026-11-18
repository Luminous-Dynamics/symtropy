#!/usr/bin/env bash
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

schema="tools/lunar-excavation-benchmark/benchmark-run.schema.json"
record="tools/lunar-excavation-benchmark/examples/synthetic-calibration-run.json"
force_trace="tools/lunar-excavation-benchmark/examples/synthetic-force-trace.csv"
model_config="tools/lunar-excavation-benchmark/examples/synthetic-model-config.json"
manifest_schema="tools/lunar-excavation-benchmark/dataset-manifest.schema.json"
manifest="tools/lunar-excavation-benchmark/examples/synthetic-dataset-manifest.json"

fail() {
  printf 'lunar benchmark fixture check: %s\n' "$*" >&2
  exit 1
}

command -v jq >/dev/null 2>&1 || fail "jq is required"

# Parse and identify the schema; this is not a general JSON Schema validator.
jq -e '.["$schema"] == "https://json-schema.org/draft/2020-12/schema"
  and .title == "Lunar Excavation Model Benchmark Run"' "$schema" >/dev/null \
  || fail "schema JSON is invalid or has an unexpected identity"

# Dataset-level split contract and synthetic group-isolation fixture.
jq -e '.["$schema"] == "https://json-schema.org/draft/2020-12/schema"
  and .title == "Lunar Excavation Benchmark Dataset Manifest"' "$manifest_schema" >/dev/null \
  || fail "dataset manifest schema JSON is invalid or has an unexpected identity"

jq -e '
  .schema_version == "0.1.0"
  and .benchmark_id == "lunar-excavation-model-validation-v0"
  and .source_class == "synthetic_fixture"
  and .split_policy.blind_test_access.hidden_from_model == true
  and .split_policy.blind_test_access.revealed_after_freeze == true
' "$manifest" >/dev/null || fail "dataset manifest does not enforce sealed blind-test semantics"

jq -e '
  [.run_assignments[].run_id] as $ids
  | ($ids | length) == ($ids | unique | length)
  and ([.run_assignments[].partition] | unique | sort) == ["blind_test", "calibration", "validation"]
' "$manifest" >/dev/null || fail "dataset manifest run IDs or partitions are incomplete"

jq -e '
  . as $m
  | ($m.split_policy.isolation_axes + $m.split_policy.optional_isolation_axes) as $axes
  | [
      $m.run_assignments[] as $r
      | $axes[] as $axis
      | $m.group_assignments[$axis][$r.group_keys[$axis]] == $r.partition
    ]
  | all(.[]; . == true)
' "$manifest" >/dev/null || fail "a run group is assigned to a different partition than its run"

jq -e '
  .split_policy.isolation_axes == [
    "material_batch_id",
    "preparation_protocol_id",
    "tool_geometry_id",
    "operating_regime_id"
  ]
  and (.split_policy.optional_isolation_axes | unique == ["campaign_id", "laboratory_id"])
' "$manifest" >/dev/null || fail "dataset split policy omits a required isolation axis"

jq -e -s '
  .[0] as $r
  | .[1] as $m
  | [$m.run_assignments[] | select(.run_id == $r.run_id)] as $matches
  | ($matches | length) == 1
    and $matches[0].partition == $r.partition
    and all(
      ($m.split_policy.isolation_axes + $m.split_policy.optional_isolation_axes)[];
      . as $axis | $r.group_keys[$axis] == $matches[0].group_keys[$axis]
    )
' "$record" "$manifest" >/dev/null || fail "run example does not match exactly one manifest assignment"

jq -e '
  .schema_version == "0.1.0"
  and .benchmark_id == "lunar-excavation-model-validation-v0"
  and .source_class == "synthetic_fixture"
  and .partition == "calibration"
  and (.notes | contains("NON-PHYSICAL SYNTHETIC FIXTURE"))
' "$record" >/dev/null || fail "example must remain an explicitly synthetic calibration fixture"

jq -e '
  [.artifacts[].artifact_id] as $ids
  | ($ids | length) == ($ids | unique | length)
' "$record" >/dev/null || fail "artifact IDs are not unique"

jq -e '
  . as $r
  | [
      $r.specimen.grain_size_distribution_artifact_id,
      $r.tool.geometry_artifact_id,
      $r.operation.tool_path_artifact_id,
      $r.observations.force_trace_artifact_id,
      $r.observations.residual_geometry_artifact_id,
      $r.model.configuration_artifact_id,
      $r.model.parameters_artifact_id
    ] as $refs
  | all($refs[]; . as $id | any($r.artifacts[]; .artifact_id == $id))
' "$record" >/dev/null || fail "an artifact reference does not resolve"

jq -e '
  .group_keys.material_batch_id == .specimen.batch_id
  and .group_keys.preparation_protocol_id == .specimen.preparation_protocol_id
  and .group_keys.tool_geometry_id == .tool.geometry_id
' "$record" >/dev/null || fail "split group keys disagree with the specimen or tool"

jq -e '
  ([.. | objects | select(has("status") and has("unit")) | .status] as $s
   | ($s | length) > 0 and all($s[]; . == "synthetic"))
  and any(.artifacts[]; (.path | startswith("synthetic-placeholder:")))
' "$record" >/dev/null || fail "fixture must remain non-physical and retain omitted-artifact sentinels"

# Confirm the included raw trace and model-config digests against the exact bytes.
printf '%s  %s\n' \
  "706b23281d0b554eef6c6edc14a72707965bdd57692d4e8478f14b28de910bbc" "$force_trace" \
  "a56e6313d1891ac46bbd6f5075fbeb768d37cdb42c4a930d4504c7d6352e2324" "$model_config" \
  | sha256sum --check --status || fail "fixture SHA-256 mismatch"

# Recompute peak, trapezoidal impulse, and time-mean force from the force trace.
read -r peak impulse mean_force < <(
  awk -F, '
    BEGIN { first = 1 }
    NR == 1 { next }
    {
      t = $1 + 0
      f = $2 + 0
      if (first) {
        first_t = t
        prev_t = t
        prev_f = f
        peak = f
        first = 0
        next
      }
      if (f > peak) peak = f
      impulse += (prev_f + f) * 0.5 * (t - prev_t)
      prev_t = t
      prev_f = f
    }
    END {
      duration = prev_t - first_t
      if (duration <= 0) exit 1
      printf "%.12f %.12f %.12f\n", peak, impulse, impulse / duration
    }
  ' "$force_trace"
) || fail "could not calculate fixture force trace"

expected_peak="$(jq -r '.observations.peak_force_N.value' "$record")"
expected_impulse="$(jq -r '.observations.impulse_N_s.value' "$record")"
expected_mean="$(jq -r '.observations.mean_force_N.value' "$record")"
feed_rate="$(jq -r '.operation.feed_rate_m_s.value' "$record")"
duration="$(jq -r '.operation.duration_s.value' "$record")"
width="$(jq -r '.tool.cutting_width_m.value' "$record")"
depth="$(jq -r '.operation.penetration_depth_m.value' "$record")"
expected_work="$(jq -r '.observations.energy_input_J.value' "$record")"
expected_volume="$(jq -r '.observations.displaced_volume_m3.value' "$record")"

close_enough() {
  awk -v actual="$1" -v expected="$2" '
    function abs(x) { return x < 0 ? -x : x }
    BEGIN {
      scale = abs(expected) > 1e-12 ? abs(expected) : 1
      exit (abs(actual - expected) <= 1e-9 * scale ? 0 : 1)
    }
  '
}

close_enough "$peak" "$expected_peak" || fail "peak force does not match raw trace"
close_enough "$impulse" "$expected_impulse" || fail "impulse does not match raw trace"
close_enough "$mean_force" "$expected_mean" || fail "mean force does not match raw trace"
close_enough "$(awk -v i="$impulse" -v v="$feed_rate" 'BEGIN { printf "%.12f", i*v }')" "$expected_work" \
  || fail "declared work does not match force impulse and feed rate"
close_enough "$(awk -v w="$width" -v d="$depth" -v v="$feed_rate" -v t="$duration" 'BEGIN { printf "%.12f", w*d*v*t }')" "$expected_volume" \
  || fail "declared displaced volume does not match the rectangular-cut fixture"

printf 'lunar benchmark fixture check: PASS (synthetic fixture only; no physical qualification)\n'

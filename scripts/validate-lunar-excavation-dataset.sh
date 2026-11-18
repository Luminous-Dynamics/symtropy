#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Dataset-level admission checks for lunar excavation benchmark run records.
# This is a semantic/provenance validator, not a general JSON Schema engine.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

usage() {
  cat >&2 <<'USAGE'
Usage:
  validate-lunar-excavation-dataset.sh [--allow-synthetic-placeholders] MANIFEST.json RUN.json [RUN.json ...]

Run/artifact paths are resolved relative to each RUN.json directory.
The placeholder option is only for committed synthetic regression fixtures.
USAGE
  exit 2
}

allow_synthetic_placeholders=false
if [[ "\${1:-}" == "--allow-synthetic-placeholders" ]]; then
  allow_synthetic_placeholders=true
  shift
fi

[[ "$#" -ge 2 ]] || usage
manifest_path="$1"
shift
run_paths=("$@")

fail() {
  printf 'lunar excavation dataset validation: FAIL: %s\n' "$*" >&2
  exit 1
}

command -v jq >/dev/null 2>&1 || fail "jq is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"
[[ -f "$manifest_path" && ! -L "$manifest_path" ]] || fail "manifest is missing or is a symlink: $manifest_path"

jq -e '
  .schema_version == "0.1.0"
  and .benchmark_id == "lunar-excavation-model-validation-v0"
  and (.dataset_id | type == "string" and length > 0)
  and (.source_class == "physical_analogue_dataset"
       or .source_class == "published_dataset"
       or .source_class == "synthetic_fixture")
  and .split_policy.isolation_axes == [
    "material_batch_id",
    "preparation_protocol_id",
    "tool_geometry_id",
    "operating_regime_id"
  ]
  and ((.split_policy.optional_isolation_axes // []) as $optional
       | all($optional[]; . == "laboratory_id" or . == "campaign_id")
       and ($optional | length == ($optional | unique | length)))
  and .split_policy.partition_order == ["calibration", "validation", "blind_test"]
  and .split_policy.blind_test_access.hidden_from_model == true
  and .split_policy.blind_test_access.revealed_after_freeze == true
' "$manifest_path" >/dev/null || fail "manifest structure or split policy is invalid"

expected_freeze='["configuration_artifact_id","executable_sha256","parameters_artifact_id","prediction_artifacts","random_seed","source_revision"]'
actual_freeze="$(jq -c '.split_policy.blind_test_access.freeze_required | sort' "$manifest_path")"
[[ "$actual_freeze" == "$expected_freeze" ]] || fail "blind-test freeze must contain all six required identity fields"

jq -e '
  [.run_assignments[].run_id] as $ids
  | ($ids | length) > 0
    and ($ids | length) == ($ids | unique | length)
    and ([.run_assignments[].partition] | unique | sort) == ["blind_test", "calibration", "validation"]
' "$manifest_path" >/dev/null || fail "manifest run IDs must be non-empty/unique and use all valid partitions"

active_axes="$(jq -c '.split_policy.isolation_axes + (.split_policy.optional_isolation_axes // [])' "$manifest_path")"
manifest_run_ids="$(jq -r '.run_assignments[].run_id' "$manifest_path" | LC_ALL=C sort)"
provided_run_ids=()

for run_path in "\${run_paths[@]}"; do
  [[ -f "$run_path" && ! -L "$run_path" ]] || fail "run record is missing or is a symlink: $run_path"

  jq -e '
    .schema_version == "0.1.0"
    and .benchmark_id == "lunar-excavation-model-validation-v0"
    and (.run_id | type == "string" and length > 0)
    and (.partition == "calibration" or .partition == "validation" or .partition == "blind_test")
    and (.group_keys | type == "object")
    and (.artifacts | type == "array" and length > 0)
    and (.model.predictions | type == "object")
  ' "$run_path" >/dev/null || fail "run record has invalid required fields: $run_path"

  run_id="$(jq -r '.run_id' "$run_path")"
  provided_run_ids+=("$run_id")

  jq -e '
    [.. | objects | select(has("status") and has("unit"))] as $measurements
    | all($measurements[];
        if .status == "unknown" then
          (has("value") and has("standard_uncertainty")
           and .value == null and .standard_uncertainty == null)
        elif .status == "measured" then
          ((.value | type) == "number")
          and ((.standard_uncertainty | type) == "number" and .standard_uncertainty >= 0)
          and (.unit | type == "string" and length > 0)
          and ((.source_artifact_id | type) == "string" and length > 0)
          and ((.calibration_record_id | type) == "string" and length > 0)
        elif .status == "published" or .status == "estimated" then
          ((.value | type) == "number")
          and ((.standard_uncertainty | type) == "number" and .standard_uncertainty >= 0)
          and (.unit | type == "string" and length > 0)
          and ((.source_artifact_id | type) == "string" and length > 0)
        elif .status == "synthetic" then
          ((.value | type) == "number")
          and ((.standard_uncertainty | type) == "number" and .standard_uncertainty >= 0)
          and (.unit | type == "string" and length > 0)
        else false end)
  ' "$run_path" >/dev/null || fail "measurement status/value/provenance rule failed: $run_path"

  jq -e '
    [.artifacts[].artifact_id] as $ids
    | ($ids | length) == ($ids | unique | length)
      and all(.artifacts[];
        (.artifact_id | type == "string" and length > 0)
        and (.path | type == "string" and length > 0)
        and (.sha256 | test("^[a-f0-9]{64}$")))
  ' "$run_path" >/dev/null || fail "artifact IDs or digests are malformed: $run_path"

  jq -e '
    . as $r
    | [.. | objects | select(has("status") and has("unit")
        and (.status == "measured" or .status == "published" or .status == "estimated"))
       | .source_artifact_id] as $sources
    | all($sources[];
        . as $id | any($r.artifacts[]; .artifact_id == $id))
    and ([.. | objects | select(has("status") and has("unit") and .status == "measured")
          | .calibration_record_id] as $calibrations
         | all($calibrations[];
             . as $id
             | any($r.artifacts[]; .artifact_id == $id and .role == "calibration_record")))
  ' "$run_path" >/dev/null || fail "source or calibration record references do not resolve: $run_path"

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
  ' "$run_path" >/dev/null || fail "required artifact reference does not resolve: $run_path"

  jq -e '
    all(.model.predictions[];
      ((.nominal_coverage | type == "number") and .nominal_coverage >= 0 and .nominal_coverage <= 1)
      and ((.lower | type == "number") and (.estimate | type == "number") and (.upper | type == "number"))
      and .lower <= .estimate and .estimate <= .upper
      and (.unit | type == "string" and length > 0))
  ' "$run_path" >/dev/null || fail "prediction interval is malformed or unordered: $run_path"

  jq -e --argjson axes "$active_axes" --slurpfile manifest "$manifest_path" '
    . as $r
    | $manifest[0] as $m
    | all($axes[];
        . as $axis
        | ($r.group_keys[$axis] | type == "string" and length > 0)
          and $m.group_assignments[$axis][$r.group_keys[$axis]] == $r.partition)
    and ($r.group_keys.material_batch_id == $r.specimen.batch_id)
    and ($r.group_keys.preparation_protocol_id == $r.specimen.preparation_protocol_id)
    and ($r.group_keys.tool_geometry_id == $r.tool.geometry_id)
  ' "$run_path" >/dev/null || fail "run group keys do not match the manifest partition map: $run_path"

  while IFS=$'\t' read -r artifact_id artifact_path artifact_sha; do
    [[ -n "$artifact_id" && -n "$artifact_path" && -n "$artifact_sha" ]] || fail "empty artifact metadata in $run_path"
    if [[ "$artifact_path" == synthetic-placeholder:* ]]; then
      [[ "$allow_synthetic_placeholders" == true ]] || fail "placeholder artifact is not admissible: $artifact_id"
      [[ "$(jq -r '.source_class' "$run_path")" == "synthetic_fixture" ]] || fail "only synthetic fixtures may carry placeholders"
      continue
    fi
    [[ "$artifact_path" != /* && "$artifact_path" != *".."* ]] || fail "artifact path must be relative and may not traverse parents: $artifact_path"
    artifact_file="$(dirname "$run_path")/$artifact_path"
    [[ -f "$artifact_file" && ! -L "$artifact_file" ]] || fail "artifact bytes are missing or symlinked: $artifact_file"
    actual_sha="$(sha256sum "$artifact_file" | awk '{print $1}')"
    [[ "$actual_sha" == "$artifact_sha" ]] || fail "artifact digest mismatch: $artifact_id"
  done < <(jq -r '.artifacts[] | [.artifact_id,.path,.sha256] | @tsv' "$run_path")
done

provided_sorted="$(mktemp)"
trap 'rm -f "$provided_sorted"' EXIT
printf '%s\n' "${provided_run_ids[@]}" | LC_ALL=C sort > "$provided_sorted"
[[ "$(uniq -d "$provided_sorted")" == "" ]] || fail "duplicate run_id supplied across run files"
[[ "$(cat "$provided_sorted")" == "$manifest_run_ids" ]] || fail "provided run files do not exactly match manifest run assignments"

for run_path in "\${run_paths[@]}"; do
  jq -e -s --argjson axes "$active_axes" '
    .[0] as $r
    | .[1] as $m
    | [$m.run_assignments[] | select(.run_id == $r.run_id)] as $matches
    | ($matches | length) == 1
      and $matches[0].partition == $r.partition
      and all($axes[];
        . as $axis | $r.group_keys[$axis] == $matches[0].group_keys[$axis])
  ' "$run_path" "$manifest_path" >/dev/null || fail "run does not match exactly one manifest assignment: $run_path"
done

printf 'lunar excavation dataset validation: PASS (%s run records; semantic/provenance checks only)\n' "\${#run_paths[@]}"

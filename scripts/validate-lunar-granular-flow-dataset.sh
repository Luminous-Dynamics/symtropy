#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Semantic/provenance admission validator for lunar granular-flow datasets.
# This is not a general JSON Schema Draft 2020-12 implementation.

set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"

usage() {
  cat >&2 <<'USAGE'
Usage:
  validate-lunar-granular-flow-dataset.sh [--allow-synthetic-placeholders] MANIFEST.json RUN.json [RUN.json ...]

Artifact paths are relative to each run JSON's directory. Placeholder paths are
permitted only for explicitly synthetic regression fixtures.
USAGE
  exit 2
}

allow_placeholders=false
if [[ "${1:-}" == "--allow-synthetic-placeholders" ]]; then
  allow_placeholders=true
  shift
fi
[[ "$#" -ge 2 ]] || usage
manifest_path="$1"
shift
run_paths=("$@")

fail() {
  printf 'lunar granular-flow validation: FAIL: %s\n' "$*" >&2
  exit 1
}

command -v jq >/dev/null 2>&1 || fail "jq is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"
command -v realpath >/dev/null 2>&1 || fail "realpath is required"
[[ -f "$manifest_path" && ! -L "$manifest_path" ]] || fail "manifest missing or symlinked: $manifest_path"

jq -e '
  .schema_version == "0.1.0"
  and .benchmark_id == "lunar-granular-flow-validation-v0"
  and (.dataset_id | type == "string" and length > 0)
  and (.source_class == "physical_analogue_dataset"
       or .source_class == "published_dataset"
       or .source_class == "synthetic_fixture")
  and .split_policy.isolation_axes == [
    "material_batch_id",
    "preparation_protocol_id",
    "wall_surface_id",
    "apparatus_geometry_id",
    "environment_profile_id",
    "operating_regime_id"
  ]
  and ((.split_policy.optional_isolation_axes // []) as $a
       | all($a[]; . == "laboratory_id" or . == "campaign_id" or . == "feeder_geometry_id" or . == "vibration_profile_id")
       and ($a | length == ($a | unique | length)))
  and .split_policy.partition_order == ["calibration", "validation", "blind_test"]
  and .split_policy.blind_test_access.hidden_from_model == true
  and .split_policy.blind_test_access.revealed_after_freeze == true
' "$manifest_path" >/dev/null || fail "manifest policy or identity invalid"

expected_freeze='["configuration_artifact_id","executable_sha256","parameters_artifact_id","prediction_artifacts","random_seed","source_revision"]'
actual_freeze="$(jq -c '.split_policy.blind_test_access.freeze_required | sort' "$manifest_path")"
[[ "$actual_freeze" == "$expected_freeze" ]] || fail "blind-test freeze identity incomplete"

jq -e '
  [.run_assignments[].run_id] as $ids
  | ($ids | length) >= 3
    and ($ids | length) == ($ids | unique | length)
    and ([.run_assignments[].partition] | unique | sort) == ["blind_test","calibration","validation"]
' "$manifest_path" >/dev/null || fail "manifest run IDs or partition coverage invalid"

active_axes="$(jq -c '.split_policy.isolation_axes + (.split_policy.optional_isolation_axes // [])' "$manifest_path")"
manifest_ids="$(jq -r '.run_assignments[].run_id' "$manifest_path" | LC_ALL=C sort)"
provided_ids=()

for run_path in "${run_paths[@]}"; do
  [[ -f "$run_path" && ! -L "$run_path" ]] || fail "run record missing or symlinked: $run_path"

  jq -e --arg source_class "$(jq -r '.source_class' "$manifest_path")" '
    .schema_version == "0.1.0"
    and .benchmark_id == "lunar-granular-flow-validation-v0"
    and .source_class == $source_class
    and (.run_id | type == "string" and length >= 3)
    and (.partition == "calibration" or .partition == "validation" or .partition == "blind_test")
    and (.group_keys | type == "object")
    and (.artifacts | type == "array" and length >= 1)
    and (.observations | type == "object")
    and (.observations.flow_continuity_status
         | . == "continuous" or . == "reduced" or . == "intermittent" or . == "no_flow" or . == "unknown")
    and (.observations.flow_interruption_count | type == "number" and . >= 0)
    and (.observations.mass_flow_rate_cv.status != null)
    and (.observations.vibration_energy_j.status != null)
    and (.observations.agglomerate_fragmentation_fraction.status != null)
    and (.operation.vibration_enabled | type == "boolean")
    and (.model.predictions | type == "object" and length >= 1)
  ' "$run_path" >/dev/null || fail "run record header or required blocks invalid: $run_path"

  run_id="$(jq -r '.run_id' "$run_path")"
  provided_ids+=("$run_id")

  jq -e '
    [.. | objects | select(has("status") and has("unit"))] as $ms
    | all($ms[];
        if .status == "unknown" then
          (has("value") and has("standard_uncertainty")
           and .value == null and .standard_uncertainty == null)
        elif .status == "measured" then
          ((.value | type) == "number")
          and ((.standard_uncertainty | type) == "number" and .standard_uncertainty >= 0)
          and (.unit | type == "string" and length > 0)
          and (.source_artifact_id | type == "string" and length > 0)
          and (.calibration_record_id | type == "string" and length > 0)
        elif .status == "published" or .status == "estimated" then
          ((.value | type) == "number")
          and ((.standard_uncertainty | type) == "number" and .standard_uncertainty >= 0)
          and (.unit | type == "string" and length > 0)
          and (.source_artifact_id | type == "string" and length > 0)
        elif .status == "synthetic" then
          ((.value | type) == "number")
          and ((.standard_uncertainty | type == "number" and .standard_uncertainty >= 0))
          and (.unit | type == "string" and length > 0)
        else false end)
  ' "$run_path" >/dev/null || fail "measurement status/value/uncertainty rule failed: $run_path"

  jq -e '
    [.artifacts[].artifact_id] as $ids
    | ($ids | length) == ($ids | unique | length)
      and all(.artifacts[];
        (.artifact_id | type == "string" and length > 0)
        and (.path | type == "string" and length > 0 and (contains("\t") | not))
        and (.sha256 | test("^[a-f0-9]{64}$")))
  ' "$run_path" >/dev/null || fail "artifact IDs, paths or digests invalid: $run_path"

  jq -e '
    . as $r
    | [
        $r.material.particle_size_distribution_artifact_id,
        $r.apparatus.geometry_artifact_id,
        $r.observations.raw_observation_artifact_id,
        $r.observations.flow_regime_observation_artifact_id,
        $r.observations.size_fraction_profile_artifact_id,
        $r.observations.composition_profile_artifact_id,
        $r.model.configuration_artifact_id,
        $r.model.parameters_artifact_id
      ] as $refs
    | all($refs[]; . as $id | any($r.artifacts[]; .artifact_id == $id))
    and ($r.observations.video_segmentation_artifact_id == null
         or any($r.artifacts[]; .artifact_id == $r.observations.video_segmentation_artifact_id))
  ' "$run_path" >/dev/null || fail "required material/apparatus/observation/model artifact reference missing: $run_path"

  jq -e '
    . as $r
    | [.. | objects | select(has("status") and has("unit")
        and (.status == "measured" or .status == "published" or .status == "estimated"))
       | .source_artifact_id] as $source_ids
    | all($source_ids[]; . as $id | any($r.artifacts[]; .artifact_id == $id))
    and ([.. | objects | select(has("status") and has("unit") and .status == "measured")
           | .calibration_record_id] as $cal_ids
         | all($cal_ids[];
             . as $id | any($r.artifacts[]; .artifact_id == $id and .role == "calibration_record")))
  ' "$run_path" >/dev/null || fail "measured/published source or calibration artifact reference does not resolve: $run_path"

  jq -e '
    if .operation.vibration_enabled then
      (.operation.vibration_gamma.value > 0)
      and (.operation.vibration_peak_acceleration_m_s2.value > 0)
      and (.environment.gravity_m_s2.value > 0)
      and (((.operation.vibration_gamma.value
             - (.operation.vibration_peak_acceleration_m_s2.value / .environment.gravity_m_s2.value))
            | fabs) <= 0.05)
    else true end
  ' "$run_path" >/dev/null || fail "vibration enablement, measured Gamma and acceleration do not reconcile: $run_path"

  jq -e '
    all(.model.predictions[];
      ((.estimate | type) == "number")
      and ((.lower | type) == "number")
      and ((.upper | type) == "number")
      and (.lower <= .estimate and .estimate <= .upper)
      and ((.nominal_coverage | type) == "number" and .nominal_coverage > 0 and .nominal_coverage <= 1)
      and (.unit | type == "string" and length > 0))
  ' "$run_path" >/dev/null || fail "prediction interval invalid or unordered: $run_path"

  jq -e --argjson axes "$active_axes" --slurpfile manifest "$manifest_path" '
    . as $r
    | $manifest[0] as $m
    | all($axes[];
        . as $axis
        | ($r.group_keys[$axis] | type == "string" and length > 0)
          and $m.group_assignments[$axis][$r.group_keys[$axis]] == $r.partition)
    and ($r.group_keys.material_batch_id == $r.material.batch_id)
    and ($r.group_keys.preparation_protocol_id == $r.material.preparation_protocol_id)
    and ($r.group_keys.wall_surface_id == $r.apparatus.wall_surface_id)
    and ($r.group_keys.apparatus_geometry_id == $r.apparatus.geometry_id)
    and ($r.group_keys.environment_profile_id == $r.environment.pressure_profile_id)
    and ($r.observations.flow_regime_classification != "unknown")
    and ($r.group_keys.feeder_geometry_id == $r.apparatus.feeder_geometry_id)
    and ($r.group_keys.vibration_profile_id == $r.operation.vibration_profile_id)
  ' "$run_path" >/dev/null || fail "run group keys do not match material, wall, apparatus or environment partition: $run_path"

  while IFS=$'\t' read -r artifact_id artifact_path artifact_sha; do
    [[ -n "$artifact_id" && -n "$artifact_path" && -n "$artifact_sha" ]] || fail "empty artifact metadata in $run_path"
    if [[ "$artifact_path" == synthetic-placeholder:* ]]; then
      [[ "$allow_placeholders" == true ]] || fail "placeholder artifact forbidden for admitted dataset: $artifact_id"
      [[ "$(jq -r '.source_class' "$run_path")" == "synthetic_fixture" ]] || fail "only synthetic run records may use placeholders"
      continue
    fi
    [[ "$artifact_path" != /* && "$artifact_path" != *".."* ]] || fail "artifact path must be relative and non-traversing: $artifact_path"
    run_dir="$(cd -- "$(dirname "$run_path")" && pwd -P)"
    candidate="$run_dir/$artifact_path"
    [[ ! -L "$candidate" ]] || fail "artifact path is a symlink: $candidate"
    artifact_file="$(realpath -e -- "$candidate")" || fail "artifact file missing: $candidate"
    case "$artifact_file" in "$run_dir"/*) ;; *) fail "artifact resolves outside the run directory: $artifact_path" ;; esac
    [[ -f "$artifact_file" ]] || fail "artifact is not a regular file: $artifact_file"
    actual_sha="$(sha256sum "$artifact_file" | awk '{print $1}')"
    [[ "$actual_sha" == "$artifact_sha" ]] || fail "artifact digest mismatch: $artifact_id"
  done < <(jq -r '.artifacts[] | [.artifact_id,.path,.sha256] | @tsv' "$run_path")
done

provided_sorted="$(mktemp)"
trap 'rm -f "$provided_sorted"' EXIT
printf '%s\n' "${provided_ids[@]}" | LC_ALL=C sort > "$provided_sorted"
[[ "$(uniq -d "$provided_sorted")" == "" ]] || fail "duplicate run IDs provided"
[[ "$(cat "$provided_sorted")" == "$manifest_ids" ]] || fail "provided run records do not exactly match manifest assignments"

for run_path in "${run_paths[@]}"; do
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

printf 'lunar granular-flow validation: PASS (%s run records; semantic/provenance checks only)\n' "${#run_paths[@]}"

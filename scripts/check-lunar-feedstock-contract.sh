#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"
mode="full"
default_run_path="tools/lunar-feedstock-contract/examples/synthetic-process-chain-run.json"
if [[ "${1:-}" == "--validate" ]]; then
  [[ "$#" -eq 2 ]] || { echo "Usage: $0 --validate RUN.json" >&2; exit 2; }
  mode="validate"
  run_path="$2"
else
  [[ "$#" -eq 0 ]] || { echo "Usage: $0" >&2; exit 2; }
  run_path="$default_run_path"
fi
schema_path="tools/lunar-feedstock-contract/feedstock-run.schema.json"

fail() {
  printf 'lunar feedstock contract: FAIL: %s\n' "$*" >&2
  exit 1
}
command -v jq >/dev/null 2>&1 || fail "jq is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"
command -v realpath >/dev/null 2>&1 || fail "realpath is required"

jq -e '
  .["$schema"] == "https://json-schema.org/draft/2020-12/schema"
  and .title == "Lunar Process-Aware Feedstock Qualification Run"
' "$schema_path" >/dev/null || fail "feedstock schema identity invalid"

[[ -f "$run_path" && ! -L "$run_path" ]] || fail "run fixture missing or symlinked"

jq -e '
  .schema_version == "0.1.0"
  and .benchmark_id == "lunar-process-aware-feedstock-v0"
  and .evidence_class == "synthetic_fixture"
  and (.run_id | type == "string" and length > 0)
  and (.acceptance_profile.profile_id | type == "string" and length > 0)
  and (.stage_events | type == "array" and length >= 1)
  and (.material_streams | type == "array" and length >= 1)
  and (.quality_samples | type == "array" and length >= 1)
  and (.artifacts | type == "array" and length >= 1)
' "$run_path" >/dev/null || fail "run identity or required collections invalid"

run_dir="$(cd -- "$(dirname "$run_path")" && pwd -P)"

jq -e '
  [.artifacts[].artifact_id] as $ids
  | ($ids | length) == ($ids | unique | length)
    and all(.artifacts[];
      (.artifact_id | type == "string" and length > 0)
      and (.path | type == "string" and length > 0 and (contains("..") | not))
      and (.sha256 | test("^[a-f0-9]{64}$")))
' "$run_path" >/dev/null || fail "artifact manifest has duplicate IDs or invalid path/digest"

while IFS=$'\t' read -r artifact_id artifact_path expected_sha; do
  [[ "$artifact_path" != /* && "$artifact_path" != *".."* ]] || fail "artifact path is absolute or traversing: $artifact_path"
  candidate="$run_dir/$artifact_path"
  [[ -f "$candidate" && ! -L "$candidate" ]] || fail "artifact missing, non-regular or symlinked: $candidate"
  canonical="$(realpath -e -- "$candidate")" || fail "artifact cannot be resolved: $candidate"
  case "$canonical" in "$run_dir"/*) ;; *) fail "artifact resolves outside fixture directory: $artifact_path" ;; esac
  actual_sha="$(sha256sum -- "$canonical" | awk '{print $1}')"
  [[ "$actual_sha" == "$expected_sha" ]] || fail "artifact digest mismatch: $artifact_id"
done < <(jq -r '.artifacts[] | [.artifact_id,.path,.sha256] | @tsv' "$run_path")

jq -e '
  . as $r
  | ([.stage_events[].event_id] | length == (unique | length))
    and ([.material_streams[].stream_id] | length == (unique | length))
    and ([.quality_samples[].sample_id] | length == (unique | length))
    and all($r.stage_events[];
      . as $event
      | ($event.started_at < $event.completed_at)
        and all($event.input_stream_ids[];
          . as $id | any($r.material_streams[]; .stream_id == $id))
        and all($event.output_stream_ids[];
          . as $id | any($r.material_streams[];
            .stream_id == $id and .created_by_event_id == $event.event_id))
        and all($event.output_stream_ids[];
          . as $id
          | ($r.material_streams | map(select(.stream_id == $id)) | .[0].parent_stream_ids) as $parents
          | ($parents | sort) == ($event.input_stream_ids | sort))
        and all($event.input_stream_ids[];
          . as $id
          | ($r.material_streams | map(select(.stream_id == $id)) | .[0].created_by_event_id) as $parent_event_id
          | ($r.stage_events | map(.event_id) | index($parent_event_id))
             < ($r.stage_events | map(.event_id) | index($event.event_id))
          and (($r.stage_events | map(select(.event_id == $parent_event_id)) | .[0].completed_at)
               <= $event.started_at))
        and any($r.artifacts[]; .artifact_id == $event.settings_artifact_id))
    and all($r.material_streams[];
      . as $stream
      | any($r.stage_events[];
          .event_id == $stream.created_by_event_id
          and any(.output_stream_ids[]; . == $stream.stream_id))
        and all($stream.parent_stream_ids[];
          . as $parent | any($r.material_streams[]; .stream_id == $parent))
        and all($stream.parent_stream_ids[];
          . as $parent
          | ($r.material_streams | map(select(.stream_id == $parent)) | .[0].created_by_event_id) as $parent_event
          | ($r.stage_events | map(.event_id) | index($parent_event))
             < ($r.stage_events | map(.event_id) | index($stream.created_by_event_id)))
        and all([$stream.composition_artifact_id,$stream.psd_artifact_id][];
          . as $artifact_id
          | $artifact_id == null
            or any($r.artifacts[]; .artifact_id == $artifact_id))
    )
' "$run_path" >/dev/null || fail "stage/stream lineage or event references invalid"

jq -e '
  . as $r
  | all($r.quality_samples[];
      . as $sample
      | any($r.material_streams[]; .stream_id == $sample.stream_id)
        and all($sample.assay_artifact_ids[];
          . as $id | any($r.artifacts[]; .artifact_id == $id))
        and ([ $sample.phase_fractions[].phase_id ] | length == (unique | length))
        and all($sample.phase_fractions[];
          (.mass_fraction.status != "unknown")
          and ((.mass_fraction.value | type) == "number")
          and ((.mass_fraction.standard_uncertainty | type) == "number")
          and (.mass_fraction.value >= 0 and .mass_fraction.value <= 1)
          and (.mass_fraction.standard_uncertainty >= 0))
        and (([$sample.phase_fractions[].mass_fraction.value] | add) as $phase_sum
             | ([$sample.phase_fractions[].mass_fraction.standard_uncertainty] | add) as $phase_uncertainty
             | (($phase_sum - 1) | abs) <= $phase_uncertainty)
        and all($sample.size_bins[];
          (.d_min_um.value > 0)
          and (.d_max_um.value > .d_min_um.value)
          and (.mass_fraction.value >= 0 and .mass_fraction.value <= 1)
          and (.mass_fraction.status != "unknown"))
        and (([$sample.size_bins[].mass_fraction.value] | add) as $size_sum
             | ([$sample.size_bins[].mass_fraction.standard_uncertainty] | add) as $size_uncertainty
             | (($size_sum - 1) | abs) <= $size_uncertainty)
        and (.target_size_mass_fraction.status != "unknown")
        and (([$sample.size_bins[]
               | select(.d_min_um.value >= $r.acceptance_profile.target_size_min_um.value
                        and .d_max_um.value <= $r.acceptance_profile.target_size_max_um.value)
               | .mass_fraction.value] | add // 0) as $derived_target_fraction
             | (($derived_target_fraction - $sample.target_size_mass_fraction.value) | abs)
               <= (0.0001 + $sample.target_size_mass_fraction.standard_uncertainty
                   + ([$sample.size_bins[]
                       | select(.d_min_um.value >= $r.acceptance_profile.target_size_min_um.value
                                and .d_max_um.value <= $r.acceptance_profile.target_size_max_um.value)
                       | .mass_fraction.standard_uncertainty] | add // 0)))
    )
' "$run_path" >/dev/null || fail "quality sample identity, assay evidence or distribution fields invalid"

assay_artifact_path="$(jq -er '.artifacts[] | select(.role == "mineral_assay") | .path' "$run_path")"
settings_artifact_path="$(jq -er '.artifacts[] | select(.role == "stage_settings") | .path' "$run_path")"
mass_artifact_path="$(jq -er '.artifacts[] | select(.role == "mass_balance") | .path' "$run_path")"

jq -e --slurpfile assay "$run_dir/$assay_artifact_path" '
  . as $r
  | all($r.quality_samples[];
      . as $sample
      | ($assay[0].samples | map(select(.sample_id == $sample.sample_id)) | .[0]) as $source
      | ($source != null)
        and ($source.stream_id == $sample.stream_id)
        and ($source.sample_point == $sample.sample_point)
        and all($sample.phase_fractions[];
          . as $phase
          | any($source.phase_fractions[];
              .phase_id == $phase.phase_id
              and .value == $phase.mass_fraction.value
              and .standard_uncertainty == $phase.mass_fraction.standard_uncertainty
              and .unit == $phase.mass_fraction.unit))
        and all($sample.size_bins[];
          . as $bin
          | any($source.size_bins[];
              .d_min_um == $bin.d_min_um.value
              and .d_max_um == $bin.d_max_um.value
              and .mass_fraction == $bin.mass_fraction.value
              and .standard_uncertainty == $bin.mass_fraction.standard_uncertainty))
        and ($source.target_size_mass_fraction == $sample.target_size_mass_fraction.value)
  )
' "$run_path" >/dev/null || fail "quality sample values do not reconcile to the referenced mineral-assay artifact"

jq -e --slurpfile settings "$run_dir/$settings_artifact_path" '
  . as $r
  | all($r.stage_events[];
      . as $event
      | any($settings[0].settings[]; .event_id == $event.event_id))
' "$run_path" >/dev/null || fail "stage event has no matching settings record in its referenced artifact"

jq -e --slurpfile mass "$run_dir/$mass_artifact_path" '
  . as $r
  | all($r.material_streams[];
      . as $stream
      | (($mass[0].streams_kg[$stream.stream_id] | type) == "number")
        and ($mass[0].streams_kg[$stream.stream_id] == $stream.measured_mass_kg.value))
' "$run_path" >/dev/null || fail "material stream mass does not reconcile to the referenced mass artifact"

# Recompute every transformation's mass balance from the referenced streams.
jq -e '
  def stream_mass($r; $id):
    [ $r.material_streams[]
      | select(.stream_id == $id)
      | .measured_mass_kg.value ][0];
  def stream_uncertainty($r; $id):
    [ $r.material_streams[]
      | select(.stream_id == $id)
      | .measured_mass_kg.standard_uncertainty ][0];
  . as $r
  | all($r.stage_events[];
      . as $event
      | if ($event.input_stream_ids | length) == 0 then
          ($event.mass_closure_status == "unknown")
          and ($event.mass_closure_residual_kg.status == "unknown")
        else
          ([$event.input_stream_ids[] as $id | stream_mass($r; $id)] | add) as $input_mass
          | ([$event.output_stream_ids[] as $id | stream_mass($r; $id)] | add) as $output_mass
          | ([$event.input_stream_ids[] as $id | (stream_uncertainty($r; $id) * stream_uncertainty($r; $id))] | add | sqrt) as $input_uncertainty
          | ([$event.output_stream_ids[] as $id | (stream_uncertainty($r; $id) * stream_uncertainty($r; $id))] | add | sqrt) as $output_uncertainty
          | ($input_mass - $output_mass) as $residual
          | sqrt(($input_uncertainty * $input_uncertainty) + ($output_uncertainty * $output_uncertainty)) as $residual_uncertainty
          | ($event.mass_closure_residual_kg.value - $residual | abs) <= ([1e-9, ($input_mass * 0.0001)] | max)
            and ($event.mass_closure_tolerance_fraction >= 0
                 and $event.mass_closure_tolerance_fraction <= 1)
            and (($residual | abs)
                 + $event.mass_closure_residual_kg.standard_uncertainty
                 + $residual_uncertainty
                 <= ($event.mass_closure_tolerance_fraction * $input_mass))
            and ($event.mass_closure_status == "within_declared_tolerance")
        end)
' "$run_path" >/dev/null || fail "one or more stage mass balances fail their declared tolerance"

# The final acceptance decision must be calculated from the reactor-inlet sample,
# not copied from a separator-outlet assay.
jq -e '
  . as $r
  | ($r.material_streams | map(select(.stream_id == $r.final_acceptance.final_stream_id)) | .[0]) as $final_stream
  | ($r.quality_samples | map(select(.sample_id == $r.final_acceptance.acceptance_sample_id)) | .[0]) as $sample
  | ($r.acceptance_profile as $profile
     | [$profile.phase_constraints[] as $constraint
        | ($sample.phase_fractions | map(select(.phase_id == $constraint.phase_id)) | .[0]) as $measurement
        | if $measurement == null or $measurement.mass_fraction.status == "unknown" then "unknown"
          else
            ($measurement.mass_fraction.value - $measurement.mass_fraction.standard_uncertainty) as $lower
            | ($measurement.mass_fraction.value + $measurement.mass_fraction.standard_uncertainty) as $upper
            | if ($constraint.minimum_mass_fraction != null and $lower < $constraint.minimum_mass_fraction)
                 and ($upper >= $constraint.minimum_mass_fraction) then "indeterminate"
              elif ($constraint.minimum_mass_fraction != null and $upper < $constraint.minimum_mass_fraction) then "out"
              elif ($constraint.maximum_mass_fraction != null and $upper > $constraint.maximum_mass_fraction)
                 and ($lower <= $constraint.maximum_mass_fraction) then "indeterminate"
              elif ($constraint.maximum_mass_fraction != null and $lower > $constraint.maximum_mass_fraction) then "out"
              else "pass" end
          end
      ] + [
        if ($sample.target_size_mass_fraction.status == "unknown") then "unknown"
        else
          ($sample.target_size_mass_fraction.value - $sample.target_size_mass_fraction.standard_uncertainty) as $lower
          | ($sample.target_size_mass_fraction.value + $sample.target_size_mass_fraction.standard_uncertainty) as $upper
          | if $lower >= $profile.minimum_target_size_mass_fraction.value then "pass"
            elif $upper < $profile.minimum_target_size_mass_fraction.value then "out"
            else "indeterminate" end
        end
      ]) as $statuses
  | ($r.stage_events | map(select((.input_stream_ids | length) > 0) | .mass_closure_status)
     | all(. == "within_declared_tolerance")) as $mass_closed
  | (if ($statuses | any(. == "unknown")) then "unknown_missing_required_measurement"
     elif ($statuses | any(. == "out")) then "qualified_out_of_spec"
     elif ($statuses | any(. == "indeterminate")) then "indeterminate_boundary_overlap"
     elif $mass_closed then "qualified_in_spec"
     else "unknown_mass_closure" end) as $expected_decision
  | ($final_stream != null)
    and ($sample != null)
    and ($sample.stream_id == $final_stream.stream_id)
    and ($sample.sample_point == "reactor_inlet")
    and ($r.acceptance_profile.process_boundary == "reactor_inlet")
    and ($final_stream.disposition == "reactor_feed")
    and ($r.final_acceptance.decision == $expected_decision)
    and ($r.final_acceptance.mass_closure_complete == $mass_closed)
    and ($r.final_acceptance.required_dimensions_complete == (all($statuses[]; . != "unknown")))
' "$run_path" >/dev/null || fail "final acceptance is not supported by reactor-inlet assays, uncertainty intervals and mass closure"

if [[ "$mode" == "validate" ]]; then
  printf 'lunar feedstock validator: PASS (%s; semantic checks only)\n' "$run_path"
  exit 0
fi

tmp_files=()
cleanup() {
  for file in "${tmp_files[@]}"; do rm -f -- "$file"; done
}
trap cleanup EXIT

make_negative() {
  local name="$1"
  local jq_filter="$2"
  local negative_path
  negative_path="$(mktemp "${run_path}.negative.XXXXXX")"
  tmp_files+=("$negative_path")
  jq "$jq_filter" "$run_path" > "$negative_path"
  if "$0" --validate "$negative_path"; then
    fail "negative control accepted: $name"
  fi
}

make_negative "separator-outlet assay reused for reactor-inlet acceptance" '
  .final_acceptance.acceptance_sample_id = "Q-SEP-001"
'

make_negative "mass output tampering hidden behind stale closure residual" '
  (.material_streams[] | select(.stream_id == "S5-REACTOR-INLET") | .measured_mass_kg.value) = 58
'

make_negative "uncertainty interval overlaps minimum phase specification but claims in-spec" '
  (.quality_samples[] | select(.sample_id == "Q-REACTOR-001")
   | .phase_fractions[] | select(.phase_id == "plagioclase") | .mass_fraction.value) = 0.705
'

make_negative "target-size fraction inconsistent with actual size-bin masses" '
  (.quality_samples[] | select(.sample_id == "Q-REACTOR-001")
   | .target_size_mass_fraction.value) = 0.80
'

make_negative "tampered expected artifact digest" '
  .artifacts[0].sha256 = ("0" + .artifacts[0].sha256[1:])
'

printf 'lunar feedstock contract: PASS (synthetic chain only; no physical process qualification)\n'

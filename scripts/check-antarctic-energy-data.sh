#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Validates a small source-attributed annual-report extract. This is a data
# lineage/accounting guard, not independent verification of the source report.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
data_file="${1:-$repo_root/research/antarctic-energy-resilience/data/aad_2024_25_annual_observations.json}"

command -v jq >/dev/null 2>&1 || {
  echo "ERROR: jq is required to audit annual Antarctic observations" >&2
  exit 2
}
test -f "$data_file" || {
  echo "ERROR: missing data extract: $data_file" >&2
  exit 2
}

audit_data() {
  local candidate="$1"
  jq -e '
    .schema_version == "antarctic-operational-energy-observations-v0.1"
    and .evidence_class == "published_aggregate_observations"
    and .source.publisher == "Australian Government Department of Climate Change, Energy, the Environment and Water"
    and .source.reporting_period == "2024–25 financial year"
    and (.source.url | contains("annual-report-2024-25.pdf"))
    and ([.observations[].id] | unique | length) == (.observations | length)
    and all(.observations[];
      (.value | type) == "number"
      and (.unit | type) == "string"
      and (.unit | length) > 0
      and (.aggregation | type) == "string"
      and (.source_table == "Table 73")
    )
    and ([.observations[] | select(.id == "aad-fy2024-25-diesel-electricity") | .value] == [5612426])
    and ([.observations[] | select(.id == "aad-fy2024-25-diesel-electricity") | .unit] == ["kWh_e"])
    and ([.observations[] | select(.id == "aad-fy2024-25-renewable-electricity") | .value] == [1004020])
    and ([.observations[] | select(.id == "aad-fy2024-25-renewable-electricity") | .unit] == ["kWh_e"])
    and ([.observations[] | select(.id == "aad-fy2024-25-operational-diesel") | .value] == [2500150])
    and ([.observations[] | select(.id == "aad-fy2024-25-operational-diesel") | .unit] == ["L"])
    and ([.observations[] | select(.id == "aad-fy2024-25-marine-fuel") | .unit] == ["L"])
    and ([.observations[] | select(.id == "aad-fy2024-25-aviation-fuel") | .unit] == ["L"])
    and ([.observations[] | select(.id == "aad-fy2024-25-water-through-2025-04-01") | .data_cutoff] == ["2025-04-01"])
    and ([.observations[] | select(.id == "aad-fy2024-25-operational-diesel") | .boundary] | first | contains("vehicles") and contains("boilers"))
    and ((.derived_metrics | length) == 1)
    and (.derived_metrics[0].status == "derived_from_reported_aggregates")
    and ((.derived_metrics[0].value - (1004020 / (5612426 + 1004020))) | abs) < 0.000000001
    and (.derived_metrics[0].unit == "fraction")
    and (.derived_metrics[0].caveat | contains("Not the share of total Antarctic energy"))
  ' "$candidate" >/dev/null
}

expect_rejected() {
  local label="$1"
  local filter="$2"
  local mutated
  mutated="$(mktemp)"
  if ! jq "$filter" "$data_file" >"$mutated"; then
    rm -f "$mutated"
    echo "ERROR: could not construct data negative control: $label" >&2
    return 1
  fi
  if audit_data "$mutated" >/dev/null 2>&1; then
    rm -f "$mutated"
    echo "ERROR: invalid data extract was accepted: $label" >&2
    return 1
  fi
  rm -f "$mutated"
  echo "PASS: rejected data negative control — $label"
}

audit_data "$data_file"
echo "PASS: source metadata, report values, units, boundaries, partial-year water label and derived ratio are consistent."

expect_rejected "changed diesel-generation value" '.observations |= map(if .id == "aad-fy2024-25-diesel-electricity" then .value = 1 else . end)'
expect_rejected "electricity unit changed to litres" '.observations |= map(if .id == "aad-fy2024-25-renewable-electricity" then .unit = "L" else . end)'
expect_rejected "duplicate observation identifier" '.observations[1].id = .observations[0].id'
expect_rejected "derived renewable share tampered" '.derived_metrics[0].value = 0.5'
expect_rejected "partial-year water label removed" '.observations |= map(if .id == "aad-fy2024-25-water-through-2025-04-01" then del(.data_cutoff) else . end)'

echo "BOUNDARY: these checks validate a source-attributed extract and arithmetic only; the source report itself and operational performance are not independently certified."
sha256sum "$data_file"

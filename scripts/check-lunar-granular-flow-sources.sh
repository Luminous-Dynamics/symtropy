#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
cd "$root"

registry="tools/lunar-granular-flow-benchmark/data-sources.json"
fail() {
  printf 'lunar granular-flow source registry: FAIL: %s\n' "$*" >&2
  exit 1
}

command -v jq >/dev/null 2>&1 || fail "jq is required"
jq -e '
  .schema_version == "0.1.0"
  and .benchmark_id == "lunar-granular-flow-validation-v0"
  and .evidence_class == "external_dataset_registry_not_ingested"
  and .policy.raw_bytes_are_never_inferred == true
  and .policy.imported_records_require_sha256 == true
  and .policy.no_dataset_is_training_eligible_until_schema_and_license_are_verified == true
  and ([.sources[].source_id] | length == (unique | length))
  and ([.sources[].source_id] | sort) == [
    "LAPPA-WATSON-BONNIEU-2026-LHS1-HOPPER",
    "STEPANOVA-DREYER-GARVEY-2026-PARABOLIC-FEEDER",
    "WYLEGALA-UHL-2026-ROTATING-DRUM"
  ]
' "$registry" >/dev/null || fail "registry identity or policy is inconsistent"

jq -e '
  [.sources[] as $source
   | $source.files[]
   | select(.status != "not_downloaded")]
  | length == 0
' "$registry" >/dev/null || fail "source registry claims an artifact was downloaded"

jq -e '
  all(.sources[];
    if .access == "author_request" then
      (.files | length == 0)
      and (.data_request_scope | length >= 3)
      and (.license_status | contains("not published as an open dataset"))
    elif .access == "public" then
      (.source_url | startswith("https://"))
      and (.files | length >= 1)
      and all(.files[];
        (.filename | type == "string" and length > 0)
        and (.display_size | type == "string" and length > 0)
        and (.status == "not_downloaded")
        and (.md5 == null or (.md5 | test("^[a-f0-9]{32}$"))))
    else false end)
' "$registry" >/dev/null || fail "source access, file metadata or licence gate is inconsistent"

jq -e '
  [.sources[] | select(.source_id == "WYLEGALA-UHL-2026-ROTATING-DRUM")][0] as $w
  | ($w.files[] | select(.file_id == "frame-level-metrics").md5)
      == "55c298d9edccc571f0ca0a3ec8e36fbd"
  and ($w.files[] | select(.file_id == "raw-and-processed-video").md5)
      == "2bf754879dc088c2cb648c5441c7d640"
  and ([.sources[] | select(.source_id == "STEPANOVA-DREYER-GARVEY-2026-PARABOLIC-FEEDER")][0].files | length) == 0
' "$registry" >/dev/null || fail "known Zenodo file checksums or author-request boundary changed"

printf 'lunar granular-flow source registry: PASS (3 sources listed; 0 datasets ingested)\n'

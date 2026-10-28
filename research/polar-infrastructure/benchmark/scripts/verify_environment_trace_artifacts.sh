#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage:
  bash verify_environment_trace_artifacts.sh MANIFEST.json SOURCE_RESPONSE.json TRACE.csv CONFIG.json

Verify source-response, normalized CSV and acquisition/transform configuration
against a local manifest using SHA-256 and byte/row-count checks. This checks
artifact integrity only; it does not validate the source data or physical model.
USAGE
  exit 64
}

[[ $# -eq 4 ]] || usage
MANIFEST=$(realpath "$1")
SOURCE_RESPONSE=$(realpath "$2")
TRACE_CSV=$(realpath "$3")
CONFIG_JSON=$(realpath "$4")

for path in "$MANIFEST" "$SOURCE_RESPONSE" "$TRACE_CSV" "$CONFIG_JSON"; do
  [[ -f "$path" && -r "$path" ]] || { echo "artifact is not a readable file: $path" >&2; exit 2; }
done
command -v jq >/dev/null
command -v sha256sum >/dev/null
command -v realpath >/dev/null

# JSON manifest fields bind to explicit local artifact arguments. Compute actual
# hashes afresh; do not trust digest strings copied into the report itself.
SOURCE_EXPECTED=$(jq -er '.acquisition.source_data_sha256 | select(type == "string" and test("^[a-f0-9]{64}$"))' "$MANIFEST")
CSV_EXPECTED=$(jq -er '.trace_asset.sha256 | select(type == "string" and test("^[a-f0-9]{64}$"))' "$MANIFEST")
CONFIG_EXPECTED=$(jq -er '.provenance.transformation_config_sha256 | select(type == "string" and test("^[a-f0-9]{64}$"))' "$MANIFEST")
SOURCE_URI_EXPECTED=$(jq -er '.acquisition.source_artifact_uri | select(type == "string" and startswith("file://"))' "$MANIFEST")
CSV_URI_EXPECTED=$(jq -er '.trace_asset.uri | select(type == "string" and startswith("file://"))' "$MANIFEST")
CSV_BYTES_EXPECTED=$(jq -er '.trace_asset.byte_size | select(type == "number" and . > 0)' "$MANIFEST")
SAMPLE_COUNT_EXPECTED=$(jq -er '.time_axis.expected_sample_count | select(type == "number" and . >= 1)' "$MANIFEST")
MANIFEST_STATUS=$(jq -er '.status | select(. == "ingested_unvalidated" or . == "validated_for_replay")' "$MANIFEST")

file_uri() {
  local encoded
  encoded=$(jq -nr --arg path "$1" '$path | @uri' | sed 's/%2F/\//g')
  printf 'file://%s' "$encoded"
}

SOURCE_URI_ACTUAL=$(file_uri "$SOURCE_RESPONSE")
CSV_URI_ACTUAL=$(file_uri "$TRACE_CSV")
[[ "$SOURCE_URI_EXPECTED" == "$SOURCE_URI_ACTUAL" ]] || {
  echo "source artifact path does not match acquisition.source_artifact_uri" >&2; exit 3;
}
[[ "$CSV_URI_EXPECTED" == "$CSV_URI_ACTUAL" ]] || {
  echo "trace CSV path does not match trace_asset.uri" >&2; exit 3;
}

SOURCE_ACTUAL=$(sha256sum -- "$SOURCE_RESPONSE" | awk '{print $1}')
CSV_ACTUAL=$(sha256sum -- "$TRACE_CSV" | awk '{print $1}')
CONFIG_ACTUAL=$(sha256sum -- "$CONFIG_JSON" | awk '{print $1}')
CSV_BYTES_ACTUAL=$(wc -c < "$TRACE_CSV" | tr -d ' ')
TRACE_ROWS_WITH_HEADER=$(wc -l < "$TRACE_CSV" | tr -d ' ')
EXPECTED_ROWS_WITH_HEADER=$((SAMPLE_COUNT_EXPECTED + 1))

[[ "$SOURCE_ACTUAL" == "$SOURCE_EXPECTED" ]] || { echo "source-response SHA-256 mismatch" >&2; exit 4; }
[[ "$CSV_ACTUAL" == "$CSV_EXPECTED" ]] || { echo "trace CSV SHA-256 mismatch" >&2; exit 4; }
[[ "$CONFIG_ACTUAL" == "$CONFIG_EXPECTED" ]] || { echo "acquisition config SHA-256 mismatch" >&2; exit 4; }
[[ "$CSV_BYTES_ACTUAL" -eq "$CSV_BYTES_EXPECTED" ]] || { echo "trace CSV byte-size mismatch" >&2; exit 4; }
[[ "$TRACE_ROWS_WITH_HEADER" -eq "$EXPECTED_ROWS_WITH_HEADER" ]] || {
  echo "trace CSV line count differs from expected sample count plus header" >&2; exit 4;
}

jq -n \
  --arg manifest "$MANIFEST" \
  --arg status "$MANIFEST_STATUS" \
  --arg source_sha256 "$SOURCE_ACTUAL" \
  --arg trace_sha256 "$CSV_ACTUAL" \
  --arg config_sha256 "$CONFIG_ACTUAL" \
  --argjson trace_bytes "$CSV_BYTES_ACTUAL" \
  --argjson trace_rows_with_header "$TRACE_ROWS_WITH_HEADER" \
  --argjson expected_samples "$SAMPLE_COUNT_EXPECTED" \
  '{
    schema_version:"0.1.0",
    report_kind:"symtropy-environment-trace-artifact-integrity-report",
    manifest_path:$manifest,
    manifest_status:$status,
    source_response_sha256:$source_sha256,
    trace_csv_sha256:$trace_sha256,
    transformation_config_sha256:$config_sha256,
    trace_csv_byte_size:$trace_bytes,
    trace_csv_rows_including_header:$trace_rows_with_header,
    expected_sample_count:$expected_samples,
    byte_hashes_and_trace_row_count_verified:true,
    result_scope:"artifact_integrity_only_not_data_quality_or_physical_model_validation"
  }'

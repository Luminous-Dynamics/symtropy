#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage:
  bash fetch_pangaea_source_file.sh DOI LICENSE_LABEL OUT_DIR

Example:
  bash fetch_pangaea_source_file.sh \
    10.1594/PANGAEA.983811 \
    "Baseline Surface Radiation Network License 1.0 (BSRN-1.0)" \
    ./artifacts/gobabeb-bsrn-2025-06

Fetches one PANGAEA dataset's tab-delimited text representation and records
source provenance. It does not parse, normalize, or validate scientific values.
Provide the license label only after checking that exact dataset's metadata.
USAGE
  exit 64
}

[[ $# -eq 3 ]] || usage
DOI=$1
LICENSE_LABEL=$2
OUT_DIR=$3

[[ "$DOI" =~ ^10[.]1594/PANGAEA[.][0-9]+$ ]] || {
  echo "DOI must match 10.1594/PANGAEA.<number>" >&2
  exit 2
}
[[ -n "$LICENSE_LABEL" ]] || { echo "license label is required; do not infer it from the series DOI" >&2; exit 2; }
command -v curl >/dev/null || { echo "curl is required" >&2; exit 69; }
command -v jq >/dev/null || { echo "jq is required" >&2; exit 69; }
command -v sha256sum >/dev/null || { echo "sha256sum is required" >&2; exit 69; }

mkdir -p "$OUT_DIR"
OUT_DIR=$(cd "$OUT_DIR" && pwd)
RAW_OUT="$OUT_DIR/pangaea-source.tsv"
META_OUT="$OUT_DIR/source-download-manifest.json"
if [[ -e "$RAW_OUT" || -e "$META_OUT" ]]; then
  echo "refusing to overwrite an existing source artifact or manifest" >&2
  exit 4
fi

TMP_RAW=$(mktemp "$OUT_DIR/.pangaea-source.XXXXXX")
trap 'rm -f "$TMP_RAW"' EXIT
REQUEST_URL="https://doi.pangaea.de/$DOI?format=textfile"
FETCHED_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)

# Keep response metadata in one tab-delimited record; curl exits non-zero for HTTP errors.
META=$(curl --fail --show-error --silent --location --retry 2 --retry-delay 1 \
  --connect-timeout 15 --max-time 180 \
  --output "$TMP_RAW" \
  --write-out $'%{content_type}\t%{url_effective}\t%{http_code}' \
  "$REQUEST_URL")

IFS=$'\t' read -r CONTENT_TYPE EFFECTIVE_URL HTTP_STATUS <<< "$META"
[[ "$HTTP_STATUS" == "200" ]] || { echo "unexpected HTTP status: $HTTP_STATUS" >&2; exit 3; }
[[ -s "$TMP_RAW" ]] || { echo "PANGAEA returned an empty source file" >&2; exit 3; }

case "$CONTENT_TYPE" in
  text/tab-separated-values*|text/plain*|application/octet-stream*) ;;
  *)
    echo "unexpected content type '$CONTENT_TYPE'; refusing to store as a text dataset" >&2
    exit 3
    ;;
esac

if head -c 65536 "$TMP_RAW" | grep -Eqi '<!doctype[[:space:]]+html|<html([[:space:]>])'; then
  echo "download appears to be HTML rather than a data file" >&2
  exit 3
fi

# Require at least one row with the publisher's ISO-like DATE/TIME first field,
# but do not parse/interpret its timezone or interval semantics here.
DATA_ROW_COUNT=$(grep -Ec '^[[:space:]]*[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}' "$TMP_RAW" || true)
[[ "$DATA_ROW_COUNT" =~ ^[0-9]+$ && "$DATA_ROW_COUNT" -gt 0 ]] || {
  echo "no timestamped PANGAEA data rows found; source format needs inspection" >&2
  exit 3
}

mv "$TMP_RAW" "$RAW_OUT"
trap - EXIT
RAW_SHA=$(sha256sum "$RAW_OUT" | awk '{print $1}')
BYTE_SIZE=$(wc -c < "$RAW_OUT" | tr -d ' ')
RAW_URI=$(jq -nr --arg path "$RAW_OUT" '"file://" + ($path | @uri | sub("^%2F"; "/") | gsub("%2F"; "/"))')
PROCESSING_REVISION=$(git rev-parse HEAD 2>/dev/null || true)
[[ -n "$PROCESSING_REVISION" ]] || PROCESSING_REVISION="unavailable_outside_git_checkout"

jq -n \
  --arg doi "$DOI" \
  --arg request_url "$REQUEST_URL" \
  --arg effective_url "$EFFECTIVE_URL" \
  --arg content_type "$CONTENT_TYPE" \
  --arg http_status "$HTTP_STATUS" \
  --arg license_label "$LICENSE_LABEL" \
  --arg fetched_at "$FETCHED_AT" \
  --arg raw_uri "$RAW_URI" \
  --arg raw_sha "$RAW_SHA" \
  --arg processing_revision "$PROCESSING_REVISION" \
  --argjson byte_size "$BYTE_SIZE" \
  --argjson data_row_count "$DATA_ROW_COUNT" \
  '{
    schema_version:"0.1.0",
    artifact_kind:"symtropy-pangaea-source-download",
    status:"source_downloaded_not_ingested",
    evidence_class:"measured_field",
    dataset_doi:$doi,
    citation_url:("https://doi.org/" + $doi),
    request_url:$request_url,
    effective_url:$effective_url,
    http_status:($http_status|tonumber),
    content_type:$content_type,
    license_label_as_checked_by_operator:$license_label,
    fetched_at_utc:$fetched_at,
    source_artifact_uri:$raw_uri,
    source_sha256:$raw_sha,
    source_byte_size:$byte_size,
    timestamped_data_row_count:$data_row_count,
    processing_code_revision:$processing_revision,
    validation_scope:"download_integrity_and_minimal_row_presence_only_not_column_mapping_timezone_QAQC_or_physical_validation",
    next_required_steps:[
      "Inspect the exact file header and metadata.",
      "Confirm the dataset-specific BSRN license and required citation.",
      "Confirm timestamp timezone, sample interval semantics, missing values and any QC flags from the actual file metadata.",
      "Hash and version any normalized output separately; preserve this source artifact unchanged."
    ]
  }' > "$META_OUT"

printf 'Downloaded PANGAEA source: %s\n' "$RAW_OUT"
printf 'Source manifest: %s\n' "$META_OUT"
printf 'DOI: %s\n' "$DOI"
printf 'Content-Type: %s\n' "$CONTENT_TYPE"
printf 'Rows with timestamp-shaped first field: %s\n' "$DATA_ROW_COUNT"
printf 'Source SHA-256: %s\n' "$RAW_SHA"
printf 'Status: source_downloaded_not_ingested (no values normalized or validated)\n'

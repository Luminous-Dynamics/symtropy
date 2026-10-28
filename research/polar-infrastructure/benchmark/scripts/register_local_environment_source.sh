#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage:
  bash register_local_environment_source.sh SOURCE_ID SOURCE_URL LICENSE_LABEL EVIDENCE_CLASS FORMAT_LABEL SOURCE_FILE OUT_DIR

Registers an already-acquired environment-data file without network access.
Supports CSV/TSV/fixed-width/other text and opaque archives. It preserves the
source bytes and records a hash, but does not authenticate origin, parse data,
review licensing, validate time/unit semantics or run scientific QA/QC.
USAGE
  exit 64
}

[[ $# -eq 7 ]] || usage
SOURCE_ID=$1
SOURCE_URL=$2
LICENSE_LABEL=$3
EVIDENCE_CLASS=$4
FORMAT_LABEL=$5
SOURCE_FILE=$6
OUT_DIR=$7

[[ "$SOURCE_ID" =~ ^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$ ]] || {
  echo "source ID must be a stable 1-128 character identifier" >&2
  exit 2
}
[[ "$SOURCE_URL" =~ ^https?://[A-Za-z0-9.-]+([:/?#][^[:space:]]*)?$ ]] || {
  echo "source URL must be an HTTP(S) URL without whitespace" >&2
  exit 2
}
[[ -n "$LICENSE_LABEL" ]] || {
  echo "license/terms label is required; record unknown terms explicitly rather than assuming permission" >&2
  exit 2
}
case "$EVIDENCE_CLASS" in
  measured_field|reanalysis|mission_observed|remote_observation|validated_simulation|uncalibrated_model|synthetic_stress|hypothesized|not_evaluated) ;;
  *) echo "unsupported evidence class '$EVIDENCE_CLASS'" >&2; exit 2 ;;
esac
case "$FORMAT_LABEL" in
  csv|tsv|fixed_width|other_text|compressed_archive|other_binary) ;;
  *) echo "unsupported format label '$FORMAT_LABEL'" >&2; exit 2 ;;
esac
[[ -f "$SOURCE_FILE" && -r "$SOURCE_FILE" && -s "$SOURCE_FILE" ]] || {
  echo "source file must be a readable, non-empty regular file" >&2
  exit 2
}
command -v jq >/dev/null || { echo "jq is required" >&2; exit 69; }
command -v sha256sum >/dev/null || { echo "sha256sum is required" >&2; exit 69; }

mkdir -p "$OUT_DIR"
OUT_DIR=$(cd "$OUT_DIR" && pwd)
RAW_OUT="$OUT_DIR/source-artifact.raw"
META_OUT="$OUT_DIR/source-registration-manifest.json"
if [[ -e "$RAW_OUT" || -e "$META_OUT" ]]; then
  echo "refusing to overwrite an existing source artifact or manifest" >&2
  exit 4
fi
SOURCE_ABS=$(cd "$(dirname "$SOURCE_FILE")" && pwd)/$(basename "$SOURCE_FILE")
cp -- "$SOURCE_ABS" "$RAW_OUT"

RAW_SHA=$(sha256sum "$RAW_OUT" | awk '{print $1}')
BYTE_SIZE=$(wc -c < "$RAW_OUT" | tr -d ' ')
REGISTERED_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)
RAW_URI=$(jq -nr --arg path "$RAW_OUT" '"file://" + ($path | @uri | sub("^%2F"; "/") | gsub("%2F"; "/"))')
PROCESSING_REVISION=$(git rev-parse HEAD 2>/dev/null || true)
[[ -n "$PROCESSING_REVISION" ]] || PROCESSING_REVISION="unavailable_outside_git_checkout"

jq -n \
  --arg source_id "$SOURCE_ID" \
  --arg source_url "$SOURCE_URL" \
  --arg license_label "$LICENSE_LABEL" \
  --arg evidence_class "$EVIDENCE_CLASS" \
  --arg format_label "$FORMAT_LABEL" \
  --arg registered_at "$REGISTERED_AT" \
  --arg source_name "$(basename "$SOURCE_FILE")" \
  --arg raw_uri "$RAW_URI" \
  --arg raw_sha "$RAW_SHA" \
  --arg processing_revision "$PROCESSING_REVISION" \
  --argjson byte_size "$BYTE_SIZE" \
  '{
    schema_version:"0.1.0",
    artifact_kind:"symtropy-environment-source-registration",
    status:"source_registered_not_ingested",
    evidence_class:$evidence_class,
    evidence_class_basis:"operator_supplied_not_independently_validated",
    acquisition_method:"offline_local_file_registration",
    source_origin_assertion:"operator_asserted_not_independently_authenticated",
    source_id:$source_id,
    source_url:$source_url,
    license_or_terms_label_as_checked_by_operator:$license_label,
    file_format_as_declared_by_operator:$format_label,
    registered_at_utc:$registered_at,
    original_filename:$source_name,
    source_artifact_uri:$raw_uri,
    source_sha256:$raw_sha,
    source_byte_size:$byte_size,
    http_status:null,
    effective_url:null,
    processing_code_revision:$processing_revision,
    validation_scope:"local_byte_integrity_only_not_origin_authentication_license_review_parsing_time_or_unit_semantics_QAQC_or_physical_validation",
    next_required_steps:[
      "Independently confirm the source identity, URL and exact dataset version.",
      "Review the applicable license or access terms before redistribution or commercial use.",
      "Inspect the complete source metadata and header before mapping fields.",
      "Validate units, timezone, interval semantics, missingness and data-quality flags.",
      "Normalize to a separate artifact with a pinned transformation and its own digest."
    ]
  }' > "$META_OUT"

printf 'Registered local environmental source: %s\n' "$RAW_OUT"
printf 'Manifest: %s\n' "$META_OUT"
printf 'Source ID: %s\n' "$SOURCE_ID"
printf 'SHA-256: %s\n' "$RAW_SHA"
printf 'Status: source_registered_not_ingested; source origin not authenticated\n'

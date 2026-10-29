#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage:
  bash register_local_pangaea_source.sh DOI LICENSE_LABEL EVIDENCE_CLASS SOURCE_FILE OUT_DIR

Registers a previously downloaded PANGAEA tab-delimited source file without
network access. This is an offline acquisition fallback for DNS/network outages.
It preserves a copy of the source bytes and writes a provenance manifest.
It does not authenticate the origin of the local file, parse scientific columns,
validate units/time semantics, or perform QA/QC.
USAGE
  exit 64
}

[[ $# -eq 5 ]] || usage
DOI=$1
LICENSE_LABEL=$2
EVIDENCE_CLASS=$3
SOURCE_FILE=$4
OUT_DIR=$5

[[ "$DOI" =~ ^10[.]1594/PANGAEA[.][0-9]+$ ]] || {
  echo "DOI must match 10.1594/PANGAEA.<number>" >&2
  exit 2
}
[[ -n "$LICENSE_LABEL" ]] || { echo "license label is required and must be reviewed against the exact dataset" >&2; exit 2; }
case "$EVIDENCE_CLASS" in
  measured_field|reanalysis|mission_observed|remote_observation|validated_simulation|uncalibrated_model|synthetic_stress|hypothesized|not_evaluated) ;;
  *) echo "unsupported evidence class '$EVIDENCE_CLASS'" >&2; exit 2 ;;
esac
[[ -f "$SOURCE_FILE" && -r "$SOURCE_FILE" && -s "$SOURCE_FILE" ]] || {
  echo "source file must be a readable, non-empty regular file" >&2
  exit 2
}
command -v jq >/dev/null || { echo "jq is required" >&2; exit 69; }
command -v sha256sum >/dev/null || { echo "sha256sum is required" >&2; exit 69; }

# PANGAEA documents this metaheader marker for its text export. This check is
# only a format plausibility gate; it does not authenticate file provenance.
if ! head -c 4096 "$SOURCE_FILE" | grep -Fq '/* DATA DESCRIPTION:'; then
  echo "source does not begin with a recognizable PANGAEA text-export metaheader" >&2
  exit 3
fi
if ! grep -Fq '*/' "$SOURCE_FILE"; then
  echo "source metaheader terminator not found" >&2
  exit 3
fi
if ! head -n 200 "$SOURCE_FILE" | grep -Fq "$DOI"; then
  echo "supplied DOI string not found in the source's opening metaheader; verify the file/DOI pair" >&2
  exit 3
fi
DATA_ROW_COUNT=$(grep -Ec '^[[:space:]]*[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}' "$SOURCE_FILE" || true)
[[ "$DATA_ROW_COUNT" =~ ^[0-9]+$ && "$DATA_ROW_COUNT" -gt 0 ]] || {
  echo "no timestamp-shaped data rows found; refusing to register as a tabular trace" >&2
  exit 3
}

mkdir -p "$OUT_DIR"
OUT_DIR=$(cd "$OUT_DIR" && pwd)
RAW_OUT="$OUT_DIR/pangaea-source.tab"
META_OUT="$OUT_DIR/source-download-manifest.json"
if [[ -e "$RAW_OUT" || -e "$META_OUT" ]]; then
  echo "refusing to overwrite an existing source artifact or manifest" >&2
  exit 4
fi
SOURCE_ABS=$(cd "$(dirname "$SOURCE_FILE")" && pwd)/$(basename "$SOURCE_FILE")
cp -- "$SOURCE_ABS" "$RAW_OUT"
RAW_SHA=$(sha256sum "$RAW_OUT" | awk '{print $1}')
BYTE_SIZE=$(wc -c < "$RAW_OUT" | tr -d ' ')
FETCHED_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)
RAW_URI=$(jq -nr --arg path "$RAW_OUT" '"file://" + ($path | @uri | sub("^%2F"; "/") | gsub("%2F"; "/"))')
PROCESSING_REVISION=$(git rev-parse HEAD 2>/dev/null || true)
[[ -n "$PROCESSING_REVISION" ]] || PROCESSING_REVISION="unavailable_outside_git_checkout"

jq -n \
  --arg doi "$DOI" \
  --arg license_label "$LICENSE_LABEL" \
  --arg evidence_class "$EVIDENCE_CLASS" \
  --arg fetched_at "$FETCHED_AT" \
  --arg raw_uri "$RAW_URI" \
  --arg raw_sha "$RAW_SHA" \
  --arg processing_revision "$PROCESSING_REVISION" \
  --arg source_name "$(basename "$SOURCE_FILE")" \
  --argjson byte_size "$BYTE_SIZE" \
  --argjson data_row_count "$DATA_ROW_COUNT" \
  '{
    schema_version:"0.1.0",
    artifact_kind:"symtropy-pangaea-source-download",
    status:"source_downloaded_not_ingested",
    evidence_class:$evidence_class,
    evidence_class_basis:"operator_supplied_not_independently_validated",
    acquisition_method:"offline_local_file_registration",
    source_origin_assertion:"operator_asserted_not_independently_authenticated",
    dataset_doi:$doi,
    citation_url:("https://doi.org/" + $doi),
    request_url:("https://doi.pangaea.de/" + $doi + "?format=textfile"),
    effective_url:null,
    http_status:null,
    content_type:"text/tab-separated-values; local MIME not independently verified",
    license_label_as_checked_by_operator:$license_label,
    fetched_at_utc:$fetched_at,
    supplied_filename:$source_name,
    source_artifact_uri:$raw_uri,
    source_sha256:$raw_sha,
    source_byte_size:$byte_size,
    timestamped_data_row_count:$data_row_count,
    processing_code_revision:$processing_revision,
    validation_scope:"local_byte_integrity_and_minimal_format_plausibility_only_not_origin_authentication_column_mapping_timezone_QAQC_or_physical_validation",
    next_required_steps:[
      "Independently confirm the DOI and provenance of the supplied file.",
      "Review the dataset-specific license and recommended citation.",
      "Inspect the actual full metaheader and column names before writing a source-specific adapter.",
      "Confirm timestamp/timezone and interval semantics, missing values and quality flags.",
      "Keep the raw artifact immutable; normalize to a separate artifact and hash it separately."
    ]
  }' > "$META_OUT"

printf 'Registered local PANGAEA source: %s\n' "$RAW_OUT"
printf 'Source manifest: %s\n' "$META_OUT"
printf 'DOI: %s\n' "$DOI"
printf 'Timestamp-shaped rows: %s\n' "$DATA_ROW_COUNT"
printf 'Source SHA-256: %s\n' "$RAW_SHA"
printf 'Status: source_downloaded_not_ingested; local origin not authenticated\n'

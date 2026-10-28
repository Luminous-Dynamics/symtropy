#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REGISTER="$SCRIPT_DIR/register_local_environment_source.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

SOURCE="$TMP_DIR/synthetic-weather.csv"
cat > "$SOURCE" <<'CSV'
date,t_mean_c,precip_mm,fog_mm
2026-01-01,22.1,0.0,0.0
2026-01-02,21.4,0.2,0.3
CSV

SOURCE_ID="synthetic.gobabeb.weather.v0"
SOURCE_URL="https://example.invalid/synthetic-weather.csv"
LICENSE="synthetic-test-only; not a real dataset licence"
OUT="$TMP_DIR/registered"
bash "$REGISTER" "$SOURCE_ID" "$SOURCE_URL" "$LICENSE" "not_evaluated" "csv" "$SOURCE" "$OUT" >/dev/null
RAW="$OUT/source-artifact.raw"
MANIFEST="$OUT/source-registration-manifest.json"

jq -e '
  .schema_version == "0.1.0" and
  .artifact_kind == "symtropy-environment-source-registration" and
  .status == "source_registered_not_ingested" and
  .evidence_class == "not_evaluated" and
  .evidence_class_basis == "operator_supplied_not_independently_validated" and
  .acquisition_method == "offline_local_file_registration" and
  .source_origin_assertion == "operator_asserted_not_independently_authenticated" and
  .source_id == "synthetic.gobabeb.weather.v0" and
  .source_url == "https://example.invalid/synthetic-weather.csv" and
  .file_format_as_declared_by_operator == "csv" and
  .http_status == null and
  .effective_url == null and
  .validation_scope == "local_byte_integrity_only_not_origin_authentication_license_review_parsing_time_or_unit_semantics_QAQC_or_physical_validation"
' "$MANIFEST" >/dev/null
test "$(sha256sum "$SOURCE" | awk '{print $1}')" = "$(jq -r '.source_sha256' "$MANIFEST")"
test "$(sha256sum "$RAW" | awk '{print $1}')" = "$(jq -r '.source_sha256' "$MANIFEST")"
cmp -- "$SOURCE" "$RAW"

if bash "$REGISTER" "bad id" "$SOURCE_URL" "$LICENSE" "not_evaluated" "csv" "$SOURCE" "$TMP_DIR/bad-id" >/dev/null 2>&1; then
  echo "ERROR: invalid source ID was unexpectedly accepted" >&2
  exit 1
fi
if bash "$REGISTER" "$SOURCE_ID" "javascript:alert(1)" "$LICENSE" "not_evaluated" "csv" "$SOURCE" "$TMP_DIR/bad-url" >/dev/null 2>&1; then
  echo "ERROR: unsupported source URL was unexpectedly accepted" >&2
  exit 1
fi
if bash "$REGISTER" "$SOURCE_ID" "$SOURCE_URL" "$LICENSE" "guess_from_domain" "csv" "$SOURCE" "$TMP_DIR/bad-class" >/dev/null 2>&1; then
  echo "ERROR: unsupported evidence class was unexpectedly accepted" >&2
  exit 1
fi
if bash "$REGISTER" "$SOURCE_ID" "$SOURCE_URL" "$LICENSE" "not_evaluated" "unknown_format" "$SOURCE" "$TMP_DIR/bad-format" >/dev/null 2>&1; then
  echo "ERROR: unsupported format label was unexpectedly accepted" >&2
  exit 1
fi
if bash "$REGISTER" "$SOURCE_ID" "$SOURCE_URL" "$LICENSE" "not_evaluated" "csv" "$SOURCE" "$OUT" >/dev/null 2>&1; then
  echo "ERROR: existing source artifact was unexpectedly overwritten" >&2
  exit 1
fi

echo "Generic offline environmental-source registration tests: PASS (synthetic input only)"

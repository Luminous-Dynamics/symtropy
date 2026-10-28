#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REGISTER="$SCRIPT_DIR/register_local_pangaea_source.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

SOURCE="$TMP_DIR/synthetic-source.tab"
cat > "$SOURCE" <<'TAB'
/* DATA DESCRIPTION:
Citation: synthetic test fixture only; not publisher observations
DOI: not applicable to fixture values
License: synthetic test-only data
*/
Date/Time	Height [m]	SWD [W/m^2]	T2 [deg C]	RH [%]
2025-06-01T00:00	2	-3	18.6	26.8
2025-06-01T00:01	2	-3	18.6	26.9
TAB

DOI="10.1594/PANGAEA.983811"
LICENSE="test-only placeholder; not a claim about dataset terms"
OUT="$TMP_DIR/registered"
bash "$REGISTER" "$DOI" "$LICENSE" "not_evaluated" "$SOURCE" "$OUT" >/dev/null
RAW="$OUT/pangaea-source.tab"
MANIFEST="$OUT/source-download-manifest.json"

jq -e '
  .schema_version == "0.1.0" and
  .artifact_kind == "symtropy-pangaea-source-download" and
  .status == "source_downloaded_not_ingested" and
  .evidence_class == "not_evaluated" and
  .evidence_class_basis == "operator_supplied_not_independently_validated" and
  .acquisition_method == "offline_local_file_registration" and
  .source_origin_assertion == "operator_asserted_not_independently_authenticated" and
  .http_status == null and
  .effective_url == null and
  .timestamped_data_row_count == 2 and
  .validation_scope == "local_byte_integrity_and_minimal_format_plausibility_only_not_origin_authentication_column_mapping_timezone_QAQC_or_physical_validation"
' "$MANIFEST" >/dev/null
test "$(sha256sum "$SOURCE" | awk '{print $1}')" = "$(jq -r '.source_sha256' "$MANIFEST")"
test "$(sha256sum "$RAW" | awk '{print $1}')" = "$(jq -r '.source_sha256' "$MANIFEST")"
cmp -- "$SOURCE" "$RAW"

if bash "$REGISTER" "bad-doi" "$LICENSE" "not_evaluated" "$SOURCE" "$TMP_DIR/bad-doi" >/dev/null 2>&1; then
  echo "ERROR: malformed DOI was unexpectedly accepted" >&2
  exit 1
fi

if bash "$REGISTER" "$DOI" "$LICENSE" "guessed_from_domain" "$SOURCE" "$TMP_DIR/bad-class" >/dev/null 2>&1; then
  echo "ERROR: unsupported evidence class was unexpectedly accepted" >&2
  exit 1
fi

if bash "$REGISTER" "$DOI" "$LICENSE" "not_evaluated" "$SOURCE" "$OUT" >/dev/null 2>&1; then
  echo "ERROR: existing output was unexpectedly overwritten" >&2
  exit 1
fi

printf 'not a PANGAEA export\n2025-06-01T00:00\t2\t10\n' > "$TMP_DIR/no-metaheader.tab"
if bash "$REGISTER" "$DOI" "$LICENSE" "not_evaluated" "$TMP_DIR/no-metaheader.tab" "$TMP_DIR/bad-format" >/dev/null 2>&1; then
  echo "ERROR: file without PANGAEA metaheader was unexpectedly accepted" >&2
  exit 1
fi

cat > "$TMP_DIR/no-rows.tab" <<'TAB'
/* DATA DESCRIPTION:
Synthetic test only
*/
Date/Time	SWD
not-a-timestamp	unknown
TAB
if bash "$REGISTER" "$DOI" "$LICENSE" "not_evaluated" "$TMP_DIR/no-rows.tab" "$TMP_DIR/no-rows" >/dev/null 2>&1; then
  echo "ERROR: source without timestamp-shaped rows was unexpectedly accepted" >&2
  exit 1
fi

echo "Offline local PANGAEA source-registration tests: PASS (synthetic inputs only)"

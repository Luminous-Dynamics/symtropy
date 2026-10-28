#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
FETCHER="$SCRIPT_DIR/fetch_pangaea_source_file.sh"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT
MOCK_BIN="$TMP_DIR/mock-bin"
mkdir -p "$MOCK_BIN"

cat > "$MOCK_BIN/curl" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
OUT=""
WRITE_OUT=""
URL=""
while (($#)); do
  case "$1" in
    --output) OUT=$2; shift 2 ;;
    --write-out) WRITE_OUT=$2; shift 2 ;;
    --retry|--retry-delay|--connect-timeout|--max-time) shift 2 ;;
    --fail|--show-error|--silent|--location) shift ;;
    *) URL=$1; shift ;;
  esac
done
[[ -n "$OUT" && -n "$WRITE_OUT" && -n "$URL" ]]
STATUS="${MOCK_CURL_STATUS:-200}"
CONTENT_TYPE="${MOCK_CURL_CONTENT_TYPE:-text/tab-separated-values}"
BODY_MODE="${MOCK_CURL_BODY_MODE:-valid}"
case "$BODY_MODE" in
  valid)
    printf '# Synthetic PANGAEA-shaped fixture, not observations\n'
    printf 'DATE/TIME\tHeight [m]\tSWD [W/m2]\n'
    printf '2025-06-01T00:00\t2\t-3\n'
    printf '2025-06-01T00:01\t2\t0.1\n'
    ;;
  html)
    printf '<!DOCTYPE html><html><body>not a dataset</body></html>\n'
    ;;
  no_rows)
    printf '# no timestamped records in this text file\nDATE/TIME\tSWD\n'
    ;;
  empty)
    : 
    ;;
  *)
    echo "unknown mock body mode" >&2
    exit 90
    ;;
esac > "$OUT"
if [[ "$STATUS" != "200" ]]; then
  exit 22
fi
printf '%s\t%s\t%s' "$CONTENT_TYPE" "$URL" "$STATUS"
MOCK
chmod +x "$MOCK_BIN/curl"
export PATH="$MOCK_BIN:$PATH"

DOI="10.1594/PANGAEA.983811"
LICENSE="BSRN License 1.0 operator-checked test label"
OUT="$TMP_DIR/valid"
bash "$FETCHER" "$DOI" "$LICENSE" "$OUT" >/dev/null

RAW="$OUT/pangaea-source.tsv"
MANIFEST="$OUT/source-download-manifest.json"
jq -e '
  .schema_version == "0.1.0" and
  .artifact_kind == "symtropy-pangaea-source-download" and
  .status == "source_downloaded_not_ingested" and
  .evidence_class == "measured_field" and
  .dataset_doi == "10.1594/PANGAEA.983811" and
  .http_status == 200 and
  .content_type == "text/tab-separated-values" and
  .timestamped_data_row_count == 2 and
  .validation_scope == "download_integrity_and_minimal_row_presence_only_not_column_mapping_timezone_QAQC_or_physical_validation"
' "$MANIFEST" >/dev/null
test "$(sha256sum "$RAW" | awk '{print $1}')" = "$(jq -r '.source_sha256' "$MANIFEST")"
test "$(wc -c < "$RAW" | tr -d ' ')" = "$(jq -r '.source_byte_size' "$MANIFEST")"

# A malformed DOI must be rejected before invoking curl.
if bash "$FETCHER" "not-a-doi" "$LICENSE" "$TMP_DIR/bad-doi" >/dev/null 2>&1; then
  echo "ERROR: invalid DOI was unexpectedly accepted" >&2
  exit 1
fi

# HTML masquerading as a dataset must be rejected and not left as a source file.
if MOCK_CURL_CONTENT_TYPE="text/html" MOCK_CURL_BODY_MODE="html" bash "$FETCHER" "$DOI" "$LICENSE" "$TMP_DIR/html" >/dev/null 2>&1; then
  echo "ERROR: HTML response was unexpectedly accepted" >&2
  exit 1
fi
test ! -e "$TMP_DIR/html/pangaea-source.tsv"

# A tabular response with no timestamped data rows must be rejected.
if MOCK_CURL_BODY_MODE="no_rows" bash "$FETCHER" "$DOI" "$LICENSE" "$TMP_DIR/no-rows" >/dev/null 2>&1; then
  echo "ERROR: response without timestamped data rows was unexpectedly accepted" >&2
  exit 1
fi

# HTTP failure must fail closed rather than create a source manifest.
if MOCK_CURL_STATUS="404" bash "$FETCHER" "$DOI" "$LICENSE" "$TMP_DIR/http-failure" >/dev/null 2>&1; then
  echo "ERROR: HTTP failure was unexpectedly accepted" >&2
  exit 1
fi

# Do not overwrite previously acquired source bytes or manifests.
if bash "$FETCHER" "$DOI" "$LICENSE" "$OUT" >/dev/null 2>&1; then
  echo "ERROR: existing source artifacts were unexpectedly overwritten" >&2
  exit 1
fi

echo "PANGAEA source downloader contract tests: PASS (mock responses only)"

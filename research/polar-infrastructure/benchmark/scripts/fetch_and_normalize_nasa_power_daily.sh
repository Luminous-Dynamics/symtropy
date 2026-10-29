#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage:
  bash fetch_and_normalize_nasa_power_daily.sh PROFILE_ID LOCATION_LABEL LAT LON START_YYYYMMDD END_YYYYMMDD OUT_DIR

Fetch a modest NASA POWER daily point request in UTC, preserve the returned JSON,
then call normalize_nasa_power_daily.sh. Outputs remain ingested_unvalidated
until independently checked against suitable reference data.
USAGE
  exit 64
}

[[ $# -eq 7 ]] || usage
PROFILE_ID=$1
LOCATION_LABEL=$2
LAT=$3
LON=$4
START=$5
END=$6
OUT_DIR=$7
[[ "$PROFILE_ID" =~ ^[a-z0-9]+([._-][a-z0-9]+)*$ ]] || { echo "invalid profile ID" >&2; exit 2; }
[[ -n "$LOCATION_LABEL" ]] || { echo "location label must be non-empty" >&2; exit 2; }
[[ "$LAT" =~ ^-?[0-9]+([.][0-9]+)?$ && "$LON" =~ ^-?[0-9]+([.][0-9]+)?$ ]] || { echo "coordinates must be numeric" >&2; exit 2; }
[[ "$START" =~ ^[0-9]{8}$ && "$END" =~ ^[0-9]{8}$ ]] || { echo "dates must use YYYYMMDD" >&2; exit 2; }
command -v curl >/dev/null || { echo "curl is required" >&2; exit 69; }

mkdir -p "$OUT_DIR"
OUT_DIR=$(cd "$OUT_DIR" && pwd)
TMP_RAW="$OUT_DIR/.nasa-power-download.json"
cleanup() { rm -f "$TMP_RAW"; }
trap cleanup EXIT

BASE_URL="https://power.larc.nasa.gov/api/temporal/daily/point"
PARAMETER_LIST="T2M,T2M_MAX,T2M_MIN,ALLSKY_SFC_SW_DWN,WS10M,PRECTOTCORR,RH2M"

# Capture the effective URL so the manifest records the exact request, not a
# generic landing page. Use a single modest point request; do not bulk-scrape
# cells or repeatedly request the same location.
EFFECTIVE_URL=$(curl --fail --show-error --silent --location --retry 2 --retry-delay 1 \
  --connect-timeout 15 --max-time 120 --get \
  --data-urlencode "parameters=$PARAMETER_LIST" \
  --data-urlencode "community=RE" \
  --data-urlencode "longitude=$LON" \
  --data-urlencode "latitude=$LAT" \
  --data-urlencode "start=$START" \
  --data-urlencode "end=$END" \
  --data-urlencode "format=JSON" \
  --data-urlencode "time-standard=UTC" \
  --output "$TMP_RAW" \
  --write-out '%{url_effective}' \
  "$BASE_URL")

[[ -s "$TMP_RAW" ]] || { echo "NASA POWER returned an empty response" >&2; exit 3; }

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
bash "$SCRIPT_DIR/normalize_nasa_power_daily.sh" \
  "$TMP_RAW" "$PROFILE_ID" "$LOCATION_LABEL" "$LAT" "$LON" \
  "$START" "$END" "$OUT_DIR" "$EFFECTIVE_URL" reanalysis

printf 'Source API: %s\n' "$BASE_URL"
printf 'Exact request URL recorded in acquisition-config.json and manifest.\n'
printf 'This is NASA POWER reanalysis; no local fog/salt/dust or equipment validation is implied.\n'

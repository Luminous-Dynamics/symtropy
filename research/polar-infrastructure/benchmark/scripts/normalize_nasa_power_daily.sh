#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage:
  bash normalize_nasa_power_daily.sh RAW_JSON PROFILE_ID LOCATION_LABEL LAT LON START_YYYYMMDD END_YYYYMMDD OUT_DIR SOURCE_URL [EVIDENCE_CLASS]

Normalize a previously downloaded NASA POWER daily point JSON response into:
  source-response.json          byte-preserved response copy; SHA-256 recorded in manifest
  environment-trace.csv         canonicalized daily values; missing values remain empty
  acquisition-config.json       acquisition/transform configuration
  environment-trace-manifest.json  ingested_unvalidated manifest

EVIDENCE_CLASS defaults to reanalysis. Use synthetic_stress only for explicitly
synthetic fixtures; synthetic values must never be labeled as NASA observations.

This script performs metadata, date-coverage, units, and value-shape checks only.
It does not validate reanalysis against station data or validate any physical model.
USAGE
  exit 64
}

[[ $# -eq 9 || $# -eq 10 ]] || usage

RAW_JSON=$1
PROFILE_ID=$2
LOCATION_LABEL=$3
LAT=$4
LON=$5
START=$6
END=$7
OUT_DIR=$8
SOURCE_URL=$9
EVIDENCE_CLASS=${10:-reanalysis}
case "$EVIDENCE_CLASS" in
  reanalysis)
    PROVIDER="NASA POWER"
    [[ "$SOURCE_URL" == https://power.larc.nasa.gov/api/temporal/daily/point\?* ]] || {
      echo "reanalysis evidence requires the exact NASA POWER daily point endpoint URL" >&2
      exit 2
    }
    ;;
  synthetic_stress) PROVIDER="synthetic test fixture" ;;
  *) echo "EVIDENCE_CLASS must be reanalysis or synthetic_stress" >&2; exit 2 ;;
esac

[[ -s "$RAW_JSON" ]] || { echo "raw NASA POWER response is missing or empty" >&2; exit 2; }
[[ "$PROFILE_ID" =~ ^[a-z0-9]+([._-][a-z0-9]+)*$ ]] || { echo "invalid profile ID" >&2; exit 2; }
[[ -n "$LOCATION_LABEL" ]] || { echo "location label must be non-empty" >&2; exit 2; }
[[ "$START" =~ ^[0-9]{8}$ && "$END" =~ ^[0-9]{8}$ ]] || { echo "dates must be YYYYMMDD" >&2; exit 2; }
[[ "$SOURCE_URL" == https://* ]] || { echo "source URL must use HTTPS" >&2; exit 2; }

START_ISO="${START:0:4}-${START:4:2}-${START:6:2}"
END_ISO="${END:0:4}-${END:4:2}-${END:6:2}"
# Requested end date is inclusive. The time axis stores the matching half-open
# interval [start, midnight after end), so N daily rows span exactly N days.
END_EXCLUSIVE_ISO=$(date -u -d "$END_ISO + 1 day" +%F 2>/dev/null || true)
[[ -n "$END_EXCLUSIVE_ISO" ]] || { echo "could not compute exclusive end date" >&2; exit 2; }
[[ "$(date -u -d "$START_ISO" +%Y%m%d 2>/dev/null || true)" == "$START" ]] || { echo "invalid start date" >&2; exit 2; }
[[ "$(date -u -d "$END_ISO" +%Y%m%d 2>/dev/null || true)" == "$END" ]] || { echo "invalid end date" >&2; exit 2; }

START_EPOCH=$(date -u -d "$START_ISO" +%s)
END_EPOCH=$(date -u -d "$END_ISO" +%s)
(( END_EPOCH >= START_EPOCH )) || { echo "end date precedes start date" >&2; exit 2; }
(( (END_EPOCH - START_EPOCH) % 86400 == 0 )) || { echo "date range is not aligned to daily UTC boundaries" >&2; exit 2; }
EXPECTED_COUNT=$(( (END_EPOCH - START_EPOCH) / 86400 + 1 ))

jq -en --arg x "$LAT" '($x | tonumber) as $n | ($n >= -90 and $n <= 90)' >/dev/null || { echo "latitude must be numeric and within [-90, 90]" >&2; exit 2; }
jq -en --arg x "$LON" '($x | tonumber) as $n | ($n >= -180 and $n <= 180)' >/dev/null || { echo "longitude must be numeric and within [-180, 180]" >&2; exit 2; }

mkdir -p "$OUT_DIR"
OUT_DIR=$(cd "$OUT_DIR" && pwd)
RAW_COPY="$OUT_DIR/source-response.json"
CSV_OUT="$OUT_DIR/environment-trace.csv"
CONFIG_OUT="$OUT_DIR/acquisition-config.json"
MANIFEST_OUT="$OUT_DIR/environment-trace-manifest.json"
cp "$RAW_JSON" "$RAW_COPY"

# NASA POWER's daily endpoint returns a GeoJSON-like Feature with parameter
# values keyed by date. Reject unexpected response layouts rather than guessing.
jq -e --arg start "$START" --arg end "$END" '
  type == "object" and
  .type == "Feature" and
  (.header | type == "object") and
  (.header.start == $start) and
  (.header.end == $end) and
  (((.header.time_standard // .header["time-standard"] // "") | ascii_upcase) == "UTC") and
  (.header.fill_value | type == "number") and
  (.geometry.type == "Point") and
  (.geometry.coordinates | type == "array" and length >= 2) and
  (.geometry.coordinates[0] | type == "number" and . >= -180 and . <= 180) and
  (.geometry.coordinates[1] | type == "number" and . >= -90 and . <= 90) and
  (.properties.parameter | type == "object") and
  (.parameters | type == "object") and
  (.messages | type == "array")
' "$RAW_COPY" >/dev/null || {
  echo "response shape, requested date range, or UTC time standard did not match; refusing normalization" >&2
  exit 3
}

PARAMETERS=(T2M T2M_MAX T2M_MIN ALLSKY_SFC_SW_DWN WS10M PRECTOTCORR RH2M)
for parameter in "${PARAMETERS[@]}"; do
  jq -e --arg p "$parameter" '
    (.properties.parameter | has($p)) and
    (.parameters | has($p)) and
    (.properties.parameter[$p] | type == "object") and
    (.parameters[$p].units | type == "string" and length > 0)
  ' "$RAW_COPY" >/dev/null || { echo "missing data or unit metadata for parameter $parameter" >&2; exit 3; }
done

# Fail closed on unit changes. The CSV transformations below are only valid
# for these declared source units; update the adapter and tests for any new unit.
jq -e '
  (.parameters.T2M.units == "C") and
  (.parameters.T2M_MAX.units == "C") and
  (.parameters.T2M_MIN.units == "C") and
  (.parameters.ALLSKY_SFC_SW_DWN.units == "kW-hr/m^2/day" or .parameters.ALLSKY_SFC_SW_DWN.units == "kWh/m^2/day") and
  (.parameters.WS10M.units == "m/s") and
  (.parameters.PRECTOTCORR.units == "mm/day") and
  (.parameters.RH2M.units == "%")
' "$RAW_COPY" >/dev/null || {
  echo "NASA POWER unit metadata differs from the adapter's reviewed unit contract" >&2
  exit 3
}

# Reject malformed/non-numeric values and basic physical contradictions before
# generating an engineering input. These are conservative sanity checks, not a
# climatological validation or calibration step.
jq -e --argjson fill "$(jq -r '.header.fill_value' "$RAW_COPY")" '
  def missing_value($value; $fill): $value == null or $value == $fill;
  def numeric_or_missing($fill): . == null or . == $fill or type == "number";
  .properties.parameter as $p |
  ($p.T2M | keys | sort) as $dates |
  all($dates[] as $d;
    all([
      $p.T2M[$d], $p.T2M_MAX[$d], $p.T2M_MIN[$d],
      $p.ALLSKY_SFC_SW_DWN[$d], $p.WS10M[$d],
      $p.PRECTOTCORR[$d], $p.RH2M[$d]
    ][];
      numeric_or_missing($fill)
    ) and
    (missing_value($p.ALLSKY_SFC_SW_DWN[$d]; $fill) or $p.ALLSKY_SFC_SW_DWN[$d] >= 0) and
    (missing_value($p.WS10M[$d]; $fill) or $p.WS10M[$d] >= 0) and
    (missing_value($p.PRECTOTCORR[$d]; $fill) or $p.PRECTOTCORR[$d] >= 0) and
    (missing_value($p.RH2M[$d]; $fill) or ($p.RH2M[$d] >= 0 and $p.RH2M[$d] <= 100)) and
    (if not missing_value($p.T2M_MIN[$d]; $fill) and not missing_value($p.T2M_MAX[$d]; $fill)
      then $p.T2M_MIN[$d] <= $p.T2M_MAX[$d]
      else true end) and
    (if not missing_value($p.T2M[$d]; $fill) and not missing_value($p.T2M_MIN[$d]; $fill) and not missing_value($p.T2M_MAX[$d]; $fill)
      then ($p.T2M_MIN[$d] <= $p.T2M[$d] and $p.T2M[$d] <= $p.T2M_MAX[$d])
      else true end)
  )
' "$RAW_COPY" >/dev/null || {
  echo "response values violate numeric, non-negativity, humidity-range, or temperature-order sanity checks" >&2
  exit 3
}

# Require all variables to use the exact same date keys and require every date
# in the inclusive request range. Missing data is represented by the API fill
# value, not by a missing date key; such values remain explicit blanks in CSV.
jq -e --argjson expected "$EXPECTED_COUNT" '
  .properties.parameter as $p |
  ($p.T2M | keys | sort) as $dates |
  ($dates | length == $expected) and
  all(["T2M","T2M_MAX","T2M_MIN","ALLSKY_SFC_SW_DWN","WS10M","PRECTOTCORR","RH2M"][];
    . as $key | (($p[$key] | keys | sort) == $dates))
' "$RAW_COPY" >/dev/null || {
  echo "date keys differ across parameters or do not cover the complete requested daily range" >&2
  exit 3
}

TMP_DATES=$(mktemp)
TMP_EXPECTED=$(mktemp)
trap 'rm -f "$TMP_DATES" "$TMP_EXPECTED"' EXIT
jq -r '.properties.parameter.T2M | keys[]' "$RAW_COPY" > "$TMP_DATES"
cursor="$START_ISO"
while [[ "$(date -u -d "$cursor" +%s)" -le "$END_EPOCH" ]]; do
  printf '%s\n' "$(date -u -d "$cursor" +%Y%m%d)" >> "$TMP_EXPECTED"
  cursor=$(date -u -d "$cursor + 1 day" +%F)
done
diff -u "$TMP_EXPECTED" "$TMP_DATES" >/dev/null || {
  echo "returned dates are not the full expected sequence" >&2
  exit 3
}

# Canonical output units: K, kWh m-2 day-1, m s-1, m water equivalent day-1,
# and relative-humidity fraction. Source JSON is retained byte-for-byte as the
# raw artifact; missing/fill values become empty CSV cells, never zero or imputed.
jq -r '
  . as $root |
  ($root.header.fill_value) as $fill |
  ($root.properties.parameter.T2M | keys | sort) as $dates |
  (["date_utc","air_temperature_mean_K","air_temperature_max_K","air_temperature_min_K","global_horizontal_irradiance_kWh_m2_day","wind_speed_10m_m_s","precipitation_m_water_equivalent_day","relative_humidity_fraction"] | @csv),
  ($dates[] as $date |
    [
      $date,
      (if $root.properties.parameter.T2M[$date] == $fill or $root.properties.parameter.T2M[$date] == null then "" else (($root.properties.parameter.T2M[$date] + 273.15) * 1000000 | round / 1000000 | tostring) end),
      (if $root.properties.parameter.T2M_MAX[$date] == $fill or $root.properties.parameter.T2M_MAX[$date] == null then "" else (($root.properties.parameter.T2M_MAX[$date] + 273.15) * 1000000 | round / 1000000 | tostring) end),
      (if $root.properties.parameter.T2M_MIN[$date] == $fill or $root.properties.parameter.T2M_MIN[$date] == null then "" else (($root.properties.parameter.T2M_MIN[$date] + 273.15) * 1000000 | round / 1000000 | tostring) end),
      (if $root.properties.parameter.ALLSKY_SFC_SW_DWN[$date] == $fill or $root.properties.parameter.ALLSKY_SFC_SW_DWN[$date] == null then "" else ($root.properties.parameter.ALLSKY_SFC_SW_DWN[$date] | tostring) end),
      (if $root.properties.parameter.WS10M[$date] == $fill or $root.properties.parameter.WS10M[$date] == null then "" else ($root.properties.parameter.WS10M[$date] | tostring) end),
      (if $root.properties.parameter.PRECTOTCORR[$date] == $fill or $root.properties.parameter.PRECTOTCORR[$date] == null then "" else (($root.properties.parameter.PRECTOTCORR[$date] / 1000) * 1000000000 | round / 1000000000 | tostring) end),
      (if $root.properties.parameter.RH2M[$date] == $fill or $root.properties.parameter.RH2M[$date] == null then "" else (($root.properties.parameter.RH2M[$date] / 100) * 1000000 | round / 1000000 | tostring) end)
    ] | @csv)
' "$RAW_COPY" > "$CSV_OUT"

FILL_VALUE=$(jq -r '.header.fill_value' "$RAW_COPY")
OBSERVED_COUNT=$(jq --argjson fill "$FILL_VALUE" '
  .properties.parameter as $p |
  ($p.T2M | keys | sort) as $dates |
  [$dates[] as $d |
    select(all([$p.T2M[$d],$p.T2M_MAX[$d],$p.T2M_MIN[$d],$p.ALLSKY_SFC_SW_DWN[$d],$p.WS10M[$d],$p.PRECTOTCORR[$d]][];
      . != null and . != $fill
    ))
  ] | length
' "$RAW_COPY")
MISSING_COUNT=$((EXPECTED_COUNT - OBSERVED_COUNT))
RETRIEVED_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)
RAW_SHA=$(sha256sum "$RAW_COPY" | awk '{print $1}')
CSV_SHA=$(sha256sum "$CSV_OUT" | awk '{print $1}')
CONFIG_SHA=""
BYTE_SIZE=$(wc -c < "$CSV_OUT" | tr -d ' ')
file_uri() {
  local encoded
  encoded=$(jq -nr --arg path "$1" '$path | @uri' | sed 's/%2F/\//g')
  printf 'file://%s' "$encoded"
}
RAW_URI=$(file_uri "$RAW_COPY")
CSV_URI=$(file_uri "$CSV_OUT")
CONFIG_URI=$(file_uri "$CONFIG_OUT")
LAT_NUM=$(jq -n --arg x "$LAT" '$x|tonumber')
LON_NUM=$(jq -n --arg x "$LON" '$x|tonumber')
ELEVATION=$(jq -r '.geometry.coordinates[2] // empty' "$RAW_COPY")
DATASET_TITLE=$(jq -r '.header.title // "NASA POWER daily point data"' "$RAW_COPY")
DATASET_VERSION=$(jq -r '.header.api.version // "not_reported_in_response"' "$RAW_COPY")
DATA_LAT=$(jq -r '.geometry.coordinates[1]' "$RAW_COPY")
DATA_LON=$(jq -r '.geometry.coordinates[0]' "$RAW_COPY")
API_SOURCES=$(jq -r '.header.sources // [] | join(", ")' "$RAW_COPY")
PROCESSING_REVISION=$(git rev-parse HEAD 2>/dev/null || true)
[[ -n "$PROCESSING_REVISION" ]] || PROCESSING_REVISION="unavailable_outside_git_checkout"

jq -n \
  --arg profile "$PROFILE_ID" \
  --arg location "$LOCATION_LABEL" \
  --arg lat "$LAT" \
  --arg lon "$LON" \
  --arg start "$START" \
  --arg end "$END" \
  --arg url "$SOURCE_URL" \
  --arg parameters "${PARAMETERS[*]}" \
  --arg api_sources "$API_SOURCES" \
  --arg data_title "$DATASET_TITLE" \
  --arg provider "$PROVIDER" \
  --arg dataset_version "$DATASET_VERSION" \
  --arg data_lat "$DATA_LAT" \
  --arg data_lon "$DATA_LON" \
  --argjson expected "$EXPECTED_COUNT" \
  --arg retrieved_at "$RETRIEVED_AT" \
  '{
    schema_version:"0.1.0",
    provider:$provider,
    dataset_title:$data_title,
    dataset_version:$dataset_version,
    source_streams:$api_sources,
    profile_id:$profile,
    location_label:$location,
    request:{
      requested_latitude:($lat|tonumber),requested_longitude:($lon|tonumber),
      response_geometry_latitude:($data_lat|tonumber),response_geometry_longitude:($data_lon|tonumber),
      start_inclusive:$start,end_inclusive:$end,timezone:"UTC",parameters:($parameters|split(" "))
    },
    source_url:$url,
    requested_daily_sample_count:$expected,
    retrieved_at_utc:$retrieved_at,
    missing_values_policy:"Preserve NASA POWER fill values as empty CSV cells; do not impute.",
    normalization:{
      T2M:"C to K by adding 273.15",
      T2M_MAX:"C to K by adding 273.15",
      T2M_MIN:"C to K by adding 273.15",
      ALLSKY_SFC_SW_DWN:"retain daily integrated irradiance in kWh m-2 day-1",
      WS10M:"retain m s-1",
      PRECTOTCORR:"mm/day to m water equivalent/day by dividing by 1000",
      RH2M:"percent to unit fraction by dividing by 100"
    }
  }' > "$CONFIG_OUT"

CONFIG_SHA=$(sha256sum "$CONFIG_OUT" | awk '{print $1}')
GEOMETRY_ELEVATION_JSON=null
if [[ -n "$ELEVATION" ]] && jq -en --arg x "$ELEVATION" '($x|tonumber) == ($x|tonumber)' >/dev/null 2>&1; then
  GEOMETRY_ELEVATION_JSON=$(jq -n --arg x "$ELEVATION" '$x|tonumber')
fi

# A trace can be source-verified/ingested without being validated for use.
# The status remains ingested_unvalidated until independent data/model checks.
jq --slurpfile template "$(dirname "$0")/../fixtures/environment-trace-manifest-template-2026.json" \
   --arg profile "$PROFILE_ID" \
   --arg location "$LOCATION_LABEL" \
   --argjson lat "$DATA_LAT" \
   --argjson lon "$DATA_LON" \
   --arg start_utc "${START_ISO}T00:00:00Z" \
   --arg end_utc "${END_EXCLUSIVE_ISO}T00:00:00Z" \
   --arg source_url "$SOURCE_URL" \
   --arg retrieved_at "$RETRIEVED_AT" \
   --arg data_version "$DATASET_VERSION" \
   --arg data_title "$DATASET_TITLE" \
   --arg provider "$PROVIDER" \
   --arg evidence_class "$EVIDENCE_CLASS" \
   --arg api_sources "$API_SOURCES" \
   --arg raw_sha "$RAW_SHA" \
   --arg csv_sha "$CSV_SHA" \
   --arg csv_uri "$CSV_URI" \
   --arg raw_uri "$RAW_URI" \
   --arg config_sha "$CONFIG_SHA" \
   --arg revision "$PROCESSING_REVISION" \
   --argjson elevation "$GEOMETRY_ELEVATION_JSON" \
   --argjson expected "$EXPECTED_COUNT" \
   --argjson observed "$OBSERVED_COUNT" \
   --argjson missing "$MISSING_COUNT" \
   --argjson byte_size "$BYTE_SIZE" \
   --arg config_uri "$CONFIG_URI" '
  $template[0] |
  .manifest_id = ("nasa_power_daily_" + $profile + "_" + .schema_version | gsub("[^a-zA-Z0-9._-]"; "_")) |
  .profile_id = $profile |
  .status = "ingested_unvalidated" |
  .evidence_class = $evidence_class |
  .acquisition = {
    provider:$provider,
    dataset_name:$data_title,
    dataset_version:$data_version,
    source_url:$source_url,
    source_artifact_uri:$raw_uri,
    attribution:(if $evidence_class == "reanalysis" then "NASA POWER; cite the API and product sources in derived research." else "Synthetic test fixture; not an observational dataset." end),
    license_or_access_terms:(if $evidence_class == "reanalysis" then "Access/redistribution terms have not been independently reviewed; consult NASA POWER documentation before redistribution." else "Test-only synthetic fixture with no external data license claim." end),
    accessed_at_utc:$retrieved_at,
    source_data_sha256:$raw_sha
  } |
  .spatial_context.location_label = $location |
  .spatial_context.latitude_deg = $lat |
  .spatial_context.longitude_deg = $lon |
  .spatial_context.elevation_m = $elevation |
  .spatial_context.spatial_support = (if $evidence_class == "reanalysis" then "NASA POWER point-response geometry; product-native resolution must be documented, not treated as an in-situ station observation" else "Synthetic fixture coordinates only; not a measured site" end) |
  .spatial_context.coordinate_reference = (if $evidence_class == "reanalysis" then "Coordinate returned in response geometry; requested coordinates and response coordinates are both preserved in acquisition configuration" else "Fixture coordinates; synthetic only" end) |
  .time_axis = {
    start_utc:$start_utc,
    end_utc:$end_utc,
    sample_interval_seconds:86400,
    expected_sample_count:$expected,
    observed_sample_count:$observed,
    missing_sample_count:$missing,
    timestamp_semantics:"calendar_day_label",
    timezone:"UTC",
    gap_policy:"preserve_and_report"
  } |
  .trace_asset = {
    uri:$csv_uri,
    sha256:$csv_sha,
    format:"text/csv",
    byte_size:$byte_size,
    immutable_archive_uri:null
  } |
  .variables = [
    {variable_id:"air_temperature_2m",physical_quantity:"daily mean near-surface air temperature at 2 m",canonical_unit:"K",source_variable:"T2M",transformation:"Add 273.15 to source Celsius values; fill values become empty cells",temporal_semantics:"interval_mean",evidence_class:$evidence_class,required_for_replay:true},
    {variable_id:"air_temperature_2m_max",physical_quantity:"daily maximum near-surface air temperature at 2 m",canonical_unit:"K",source_variable:"T2M_MAX",transformation:"Add 273.15 to source Celsius values; fill values become empty cells",temporal_semantics:"interval_maximum",evidence_class:$evidence_class,required_for_replay:true},
    {variable_id:"air_temperature_2m_min",physical_quantity:"daily minimum near-surface air temperature at 2 m",canonical_unit:"K",source_variable:"T2M_MIN",transformation:"Add 273.15 to source Celsius values; fill values become empty cells",temporal_semantics:"interval_minimum",evidence_class:$evidence_class,required_for_replay:true},
    {variable_id:"global_horizontal_irradiance",physical_quantity:"daily integrated all-sky surface shortwave irradiance",canonical_unit:"kWh m-2 day-1",source_variable:"ALLSKY_SFC_SW_DWN",transformation:"Retain daily integrated source value in kWh m-2 day-1",temporal_semantics:"interval_accumulation",evidence_class:$evidence_class,required_for_replay:true},
    {variable_id:"wind_speed_10m",physical_quantity:"daily mean wind speed at 10 m",canonical_unit:"m s-1",source_variable:"WS10M",transformation:"Retain m/s source values; fill values become empty cells",temporal_semantics:"interval_mean",evidence_class:$evidence_class,required_for_replay:true},
    {variable_id:"precipitation_amount",physical_quantity:"daily corrected precipitation amount",canonical_unit:"m water equivalent day-1",source_variable:"PRECTOTCORR",transformation:"Convert mm/day to m water equivalent/day by dividing by 1000",temporal_semantics:"interval_accumulation",evidence_class:$evidence_class,required_for_replay:true},
    {variable_id:"relative_humidity_2m",physical_quantity:"daily mean relative humidity at 2 m",canonical_unit:"1 (fraction)",source_variable:"RH2M",transformation:"Convert source percent to fraction by dividing by 100",temporal_semantics:"interval_mean",evidence_class:$evidence_class,required_for_replay:false},
    {variable_id:"fog_liquid_water_content",physical_quantity:"fog liquid water content",canonical_unit:"kg m-3",source_variable:null,transformation:"Not supplied by this POWER request; requires local measurements or a validated derivation",temporal_semantics:"unknown",evidence_class:"not_evaluated",required_for_replay:false},
    {variable_id:"salt_deposition_flux",physical_quantity:"surface salt deposition mass flux",canonical_unit:"kg m-2 s-1",source_variable:null,transformation:"Not supplied by this POWER request; requires local measurements or a validated model",temporal_semantics:"unknown",evidence_class:"not_evaluated",required_for_replay:false},
    {variable_id:"dust_deposition_flux",physical_quantity:"surface dust deposition mass flux",canonical_unit:"kg m-2 s-1",source_variable:null,transformation:"Not supplied by this POWER request; requires local measurements or a validated model",temporal_semantics:"unknown",evidence_class:"not_evaluated",required_for_replay:false}
  ] |
  .quality = {
    validation_status:"screened",
    missing_data_policy:"Preserve fill/missing values as empty CSV cells and report days complete across all required replay variables; optional-variable missingness remains visible in per-column blanks. Do not impute or treat missing values as zero.",
    known_biases_or_limits:[
      ("Evidence class is " + $evidence_class + "; this manifest does not itself validate the physical data source."),
      "Daily averages/extrema/accumulations do not resolve intra-day power ramps or controller dynamics.",
      "Fog liquid-water content, salt deposition, dust deposition, equipment response and local microclimate are not validated by this trace.",
      ("NASA POWER source streams: " + $api_sources),
      ("Acquisition configuration: " + $config_uri)
    ],
    uncertainty_method:"Not estimated by this acquisition step; record source-specific uncertainty and validate against appropriately matched observations before use.",
    validation_evidence_uri:null,
    validated_against:[]
  } |
  .provenance = {
    processing_code_revision:$revision,
    transformation_recipe:"normalize_nasa_power_daily.sh; preserve raw JSON; verify UTC/date-key coverage and source units; convert temperature to K, precipitation to m/day and relative humidity to fraction; leave fill values blank.",
    transformation_config_sha256:$config_sha,
    created_at_utc:$retrieved_at
  }
' > "$MANIFEST_OUT"

printf 'Created source response: %s\n' "$RAW_COPY"
printf 'Created normalized trace: %s\n' "$CSV_OUT"
printf 'Created manifest: %s\n' "$MANIFEST_OUT"
printf 'Evidence class: %s\n' "$EVIDENCE_CLASS"
printf 'Evidence status: ingested_unvalidated (no station/model validation claimed)\n'
printf 'Rows requested: %s; complete rows: %s; incomplete rows: %s\n' "$EXPECTED_COUNT" "$OBSERVED_COUNT" "$MISSING_COUNT"
printf 'Raw SHA-256: %s\nNormalized CSV SHA-256: %s\n' "$RAW_SHA" "$CSV_SHA"

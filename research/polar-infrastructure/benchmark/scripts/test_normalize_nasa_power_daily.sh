#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
BENCHMARK_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
NORMALIZER="$SCRIPT_DIR/normalize_nasa_power_daily.sh"
FIXTURE="$BENCHMARK_DIR/fixtures/nasa-power-daily-response-synthetic-2026.json"
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

command -v jq >/dev/null
command -v sha256sum >/dev/null
command -v date >/dev/null
[[ -s "$FIXTURE" ]]
jq -e '.header.title | contains("Synthetic")' "$FIXTURE" >/dev/null

# This fixture follows the response envelope but its values are explicitly
# synthetic. The evidence class must remain synthetic_stress end to end.
bash "$NORMALIZER" \
  "$FIXTURE" \
  "earth.coastal_desert_namib" \
  "Synthetic fixture location; not a measured Namib site" \
  "-23.56" \
  "15.04" \
  "20240101" \
  "20240103" \
  "$TMP_DIR/ok" \
  "https://example.invalid/synthetic-fixture-not-a-data-source" \
  "synthetic_stress"

RAW="$TMP_DIR/ok/source-response.json"
CSV="$TMP_DIR/ok/environment-trace.csv"
MANIFEST="$TMP_DIR/ok/environment-trace-manifest.json"
CONFIG="$TMP_DIR/ok/acquisition-config.json"

jq -e '
  .schema_version == "0.1.0" and
  .artifact_kind == "symtropy-environment-trace-manifest" and
  .status == "ingested_unvalidated" and
  .evidence_class == "synthetic_stress" and
  .acquisition.provider == "synthetic test fixture" and
  (.acquisition.source_artifact_uri | startswith("file://")) and
  .acquisition.attribution == "Synthetic test fixture; not an observational dataset." and
  .spatial_context.latitude_deg == -23.56 and
  .spatial_context.longitude_deg == 15.04 and
  .time_axis.start_utc == "2024-01-01T00:00:00Z" and
  .time_axis.end_utc == "2024-01-04T00:00:00Z" and
  .time_axis.sample_interval_seconds == 86400 and
  .time_axis.expected_sample_count == 3 and
  .time_axis.observed_sample_count == 2 and
  .time_axis.missing_sample_count == 1 and
  .time_axis.timestamp_semantics == "calendar_day_label" and
  .trace_asset.immutable_archive_uri == null and
  any(.variables[]; .variable_id == "air_temperature_2m_max" and .evidence_class == "synthetic_stress" and .temporal_semantics == "interval_maximum") and
  any(.variables[]; .variable_id == "dust_deposition_flux" and .evidence_class == "not_evaluated" and .source_variable == null) and
  .quality.validation_status == "screened" and
  (.quality.validated_against | length == 0)
' "$MANIFEST" >/dev/null

# Assert explicit conversions and ensure a fill value remains blank rather than
# becoming zero. Daily totals cannot be read as instantaneous power values.
grep -Fx '"date_utc","air_temperature_mean_K","air_temperature_max_K","air_temperature_min_K","global_horizontal_irradiance_kWh_m2_day","wind_speed_10m_m_s","precipitation_m_water_equivalent_day","relative_humidity_fraction"' "$CSV" >/dev/null
grep -Fx '"20240101","293.15","303.15","283.15","6.5","3.2","0","0.6"' "$CSV" >/dev/null
grep -Fx '"20240102","294.15","","284.15","5.0","4.0","0.0003","","' "$CSV" >/dev/null
grep -Fx '"20240103","295.15","304.15","285.15","6.2","2.1","0","0.55"' "$CSV" >/dev/null
! grep -q -- '-999' "$CSV"

# Independently verify hashes, manifest-to-file binding, CSV byte size and row count.
VERIFY_REPORT=$(bash "$SCRIPT_DIR/verify_environment_trace_artifacts.sh" "$MANIFEST" "$RAW" "$CSV" "$CONFIG")
jq -e '
  .report_kind == "symtropy-environment-trace-artifact-integrity-report" and
  .byte_hashes_and_trace_row_count_verified == true and
  .expected_sample_count == 3 and
  .trace_csv_rows_including_header == 4 and
  .result_scope == "artifact_integrity_only_not_data_quality_or_physical_model_validation"
' <<<"$VERIFY_REPORT" >/dev/null
jq -e '.request.requested_daily_sample_count == 3 and .request.start_inclusive == "20240101" and .request.end_inclusive == "20240103" and .request.timezone == "UTC"' "$CONFIG" >/dev/null

# Negative control: modify one trace byte but keep the old digest. The verifier
# must fail even after the supplied path is rebound to the tampered file URI.
sed 's/293.15/293.16/' "$CSV" > "$TMP_DIR/tampered.csv"
jq --arg uri "file://$TMP_DIR/tampered.csv" '.trace_asset.uri = $uri' "$MANIFEST" > "$TMP_DIR/tampered-manifest.json"
if bash "$SCRIPT_DIR/verify_environment_trace_artifacts.sh" "$TMP_DIR/tampered-manifest.json" "$RAW" "$TMP_DIR/tampered.csv" "$CONFIG" >/dev/null 2>&1; then
  echo "ERROR: tampered trace bytes were unexpectedly accepted" >&2
  exit 1
fi

# Check the Rust manifest gate against the emitted test-only manifest when a
# binary path is supplied by CI. The shell adapter tests stay independently
# runnable on machines without Rust.
if [[ -n "${TRACE_MANIFEST_BIN:-}" ]]; then
  "$TRACE_MANIFEST_BIN" "$MANIFEST" > "$TMP_DIR/rust-report.json"
  jq -e '.contract_valid == true and .candidate_for_independent_verification == false and .manifest_status == "ingested_unvalidated"' "$TMP_DIR/rust-report.json" >/dev/null
fi

# Negative control: a plausible-looking but wrong unit contract must be rejected.
jq '.parameters.T2M.units = "K"' "$FIXTURE" > "$TMP_DIR/wrong-units.json"
if bash "$NORMALIZER" \
  "$TMP_DIR/wrong-units.json" \
  "earth.coastal_desert_namib" \
  "Synthetic fixture" "-23.56" "15.04" \
  "20240101" "20240103" "$TMP_DIR/wrong-unit-out" \
  "https://example.invalid/synthetic-fixture-not-a-data-source" \
  "synthetic_stress" >/dev/null 2>&1; then
  echo "ERROR: wrong parameter units were unexpectedly accepted" >&2
  exit 1
fi

# Negative control: negative precipitation is a malformed physical value, not a valid weather trace.
jq '.properties.parameter.PRECTOTCORR["20240102"] = -0.1' "$FIXTURE" > "$TMP_DIR/negative-precip.json"
if bash "$NORMALIZER" \
  "$TMP_DIR/negative-precip.json" \
  "earth.coastal_desert_namib" \
  "Synthetic fixture" "-23.56" "15.04" \
  "20240101" "20240103" "$TMP_DIR/negative-precip-out" \
  "https://example.invalid/synthetic-fixture-not-a-data-source" \
  "synthetic_stress" >/dev/null 2>&1; then
  echo "ERROR: negative precipitation was unexpectedly accepted" >&2
  exit 1
fi

# Negative control: relative humidity cannot exceed 100 percent.
jq '.properties.parameter.RH2M["20240102"] = 101' "$FIXTURE" > "$TMP_DIR/humidity-over-100.json"
if bash "$NORMALIZER" \
  "$TMP_DIR/humidity-over-100.json" \
  "earth.coastal_desert_namib" \
  "Synthetic fixture" "-23.56" "15.04" \
  "20240101" "20240103" "$TMP_DIR/humidity-out" \
  "https://example.invalid/synthetic-fixture-not-a-data-source" \
  "synthetic_stress" >/dev/null 2>&1; then
  echo "ERROR: relative humidity above 100 percent was unexpectedly accepted" >&2
  exit 1
fi

# Negative control: the daily mean must remain between the source daily minimum
# and maximum when all three are present.
jq '.properties.parameter.T2M["20240101"] = 35' "$FIXTURE" > "$TMP_DIR/inconsistent-temperature.json"
if bash "$NORMALIZER" \
  "$TMP_DIR/inconsistent-temperature.json" \
  "earth.coastal_desert_namib" \
  "Synthetic fixture" "-23.56" "15.04" \
  "20240101" "20240103" "$TMP_DIR/temperature-out" \
  "https://example.invalid/synthetic-fixture-not-a-data-source" \
  "synthetic_stress" >/dev/null 2>&1; then
  echo "ERROR: daily mean above daily maximum was unexpectedly accepted" >&2
  exit 1
fi

# Negative control: local-solar-time response must not pass as UTC.
jq '.header.time_standard = "LST"' "$FIXTURE" > "$TMP_DIR/lst-response.json"
if bash "$NORMALIZER" \
  "$TMP_DIR/lst-response.json" \
  "earth.coastal_desert_namib" \
  "Synthetic fixture" "-23.56" "15.04" \
  "20240101" "20240103" "$TMP_DIR/lst-out" \
  "https://example.invalid/synthetic-fixture-not-a-data-source" \
  "synthetic_stress" >/dev/null 2>&1; then
  echo "ERROR: LST response was unexpectedly accepted as UTC" >&2
  exit 1
fi

echo "NASA POWER daily trace adapter tests: PASS (synthetic fixture only)"

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
grep -Fx '"20240102","294.15","","284.15","5","4","0.0003","","' "$CSV" >/dev/null
grep -Fx '"20240103","295.15","304.15","285.15","6.2","2.1","0","0.55"' "$CSV" >/dev/null
! grep -q -- '-999' "$CSV"

# The raw and transformed artifacts must match their recorded byte digests.
test "$(sha256sum "$RAW" | awk '{print $1}')" = "$(jq -r '.acquisition.source_data_sha256' "$MANIFEST")"
test "$(sha256sum "$CSV" | awk '{print $1}')" = "$(jq -r '.trace_asset.sha256' "$MANIFEST")"
test "$(sha256sum "$CONFIG" | awk '{print $1}')" = "$(jq -r '.provenance.transformation_config_sha256' "$MANIFEST")"
jq -e '.request.requested_daily_sample_count == 3 and .request.start_inclusive == "20240101" and .request.end_inclusive == "20240103" and .request.timezone == "UTC"' "$CONFIG" >/dev/null

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

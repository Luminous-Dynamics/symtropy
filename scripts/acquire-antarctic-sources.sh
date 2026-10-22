#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Local source acquisition helper. Raw bytes remain outside the git worktree.
# A successful fetch means bytes were retrieved, not that their scientific
# contents, units, calibration, completeness or model applicability were proven.

set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/acquire-antarctic-sources.sh list-usgs
  scripts/acquire-antarctic-sources.sh metadata
  scripts/acquire-antarctic-sources.sh fetch-aadc-fuel
  scripts/acquire-antarctic-sources.sh fetch-aadc-electricity
  scripts/acquire-antarctic-sources.sh fetch-url SOURCE_ID HTTPS_URL FILENAME LICENSE

Default cache: ${XDG_CACHE_HOME:-~/.cache}/luminous-dynamics/antarctic-evidence
AADC downloads require AADC_ORIGINATOR_CONTACT_CONFIRMED=1 after following the
originator-contact instruction in the dataset citation page.
USGS release is declared CC0 1.0; only fetch files actually listed by USGS.
Raw files are never added to git automatically.
USAGE
}

for cmd in curl jq sha256sum; do
  command -v "$cmd" >/dev/null 2>&1 || { echo "ERROR: missing required command: $cmd" >&2; exit 2; }
done

cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/luminous-dynamics/antarctic-evidence"
mkdir -p "$cache_root/raw" "$cache_root/metadata" "$cache_root/manifests"
manifest="$cache_root/manifests/acquisition.jsonl"

record() {
  local source_id="$1" url="$2" final_url="$3" status="$4" ctype="$5" file="$6" result="$7"
  local size=0 digest=""
  if [[ -f "$file" ]]; then
    size="$(wc -c < "$file" | tr -d ' ')"
    digest="$(sha256sum "$file" | awk '{print $1}')"
  fi
  jq -cn --arg id "$source_id" --arg url "$url" --arg final "$final_url" \
    --arg status "$status" --arg ctype "$ctype" --arg file "$file" \
    --arg result "$result" --arg at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
    --arg sha "$digest" --argjson size "$size" \
    '{source_id:$id,requested_url:$url,effective_url:$final,http_status:$status,
      content_type:$ctype,path:$file,bytes:$size,sha256:($sha|select(length>0)),
      retrieved_at_utc:$at,result:$result}' >> "$manifest"
}

fetch_one() {
  local id="$1" url="$2" path="$3" license="$4"
  local tmp="${path}.part" info status ctype effective
  mkdir -p "$(dirname "$path")"
  [[ ! -e "$path" ]] || { echo "ERROR: refusing to overwrite $path" >&2; return 2; }
  info="$(mktemp)"
  if ! curl --fail --location --silent --show-error --retry 2 \
      --output "$tmp" --write-out $'\n%{http_code}\t%{content_type}\t%{url_effective}\n' \
      "$url" > "$info"; then
    record "$id" "$url" "$url" "transport-failed" "" "$tmp" "failed"
    rm -f "$tmp" "$info"
    echo "ERROR: retrieval failed; inspect $manifest" >&2
    return 4
  fi
  status="$(tail -n 1 "$info" | cut -f1)"
  ctype="$(tail -n 1 "$info" | cut -f2)"
  effective="$(tail -n 1 "$info" | cut -f3-)"
  if [[ "$ctype" =~ [Tt][Ee][Xx][Tt]/[Hh][Tt][Mm][Ll] ]] || \
     head -c 256 "$tmp" | grep -Eiq '<!doctype[[:space:]]+html|<html([[:space:]>])'; then
    record "$id" "$url" "$effective" "$status" "$ctype" "$tmp" "rejected-html-not-data"
    rm -f "$tmp" "$info"
    echo "ERROR: received HTML instead of a source artifact for $id" >&2
    return 3
  fi
  [[ -s "$tmp" ]] || {
    record "$id" "$url" "$effective" "$status" "$ctype" "$tmp" "rejected-empty"
    rm -f "$tmp" "$info"
    echo "ERROR: zero-byte response for $id" >&2
    return 3
  }
  mv "$tmp" "$path"
  record "$id" "$url" "$effective" "$status" "$ctype" "$path" "retrieved-unvalidated"
  rm -f "$info"
  printf 'Retrieved candidate (not validated): %s\n' "$path"
  printf 'HTTP %s | Content-Type %s | declared terms %s\n' "$status" "$ctype" "$license"
  printf 'SHA-256 %s\n' "$(sha256sum "$path" | awk '{print $1}')"
}

contact_gate() {
  if [[ "${AADC_ORIGINATOR_CONTACT_CONFIRMED:-0}" != "1" ]]; then
    echo "BLOCKED: follow the AADC citation instruction to contact the data originator before applying these data." >&2
    echo "https://data.aad.gov.au/metadata/records/SOE_generator_boiler_fuel_usage/citation" >&2
    return 5
  fi
}

case "${1:-}" in
  --help|-h|help) usage ;;
  list-usgs)
    url="https://www.sciencebase.gov/catalog/item/698deeecb66b01f8cb40ec1b?format=json"
    path="$cache_root/metadata/usgs-larsen-release-item.json"
    fetch_one "usgs-larsen-release-metadata" "$url" "$path" "CC0-1.0-as-declared-by-USGS"
    jq -r '.files[]? | [(.name // ""), ((.size // 0)|tostring), (.url // "")] | @tsv' "$path"
    ;;
  metadata)
    fetch_one "aadc-fuel-metadata" \
      "https://data.aad.gov.au/metadata/records/fulldisplay/SOE_generator_boiler_fuel_usage" \
      "$cache_root/metadata/aadc-fuel-metadata.html" "public metadata"
    fetch_one "aadc-electricity-metadata" \
      "https://data.aad.gov.au/metadata/records/SOE_SFU" \
      "$cache_root/metadata/aadc-electricity-metadata.html" "public metadata"
    fetch_one "usgs-larsen-release-landing" \
      "https://www.usgs.gov/data/usgs-national-and-global-oil-and-gas-assessment-project-larsen-basin-antarctica-assessment" \
      "$cache_root/metadata/usgs-larsen-release-landing.html" "CC0-1.0-as-declared-by-USGS"
    ;;
  fetch-aadc-fuel)
    contact_gate
    fetch_one "aadc-generator-boiler-fuel-monthly" \
      "https://data.aad.gov.au/eds/4936/download" \
      "$cache_root/raw/aadc-generator-boiler-fuel-monthly.bin" "CC-BY-4.0; contact instruction applies"
    ;;
  fetch-aadc-electricity)
    contact_gate
    fetch_one "aadc-electricity-monthly-candidate" \
      "https://data.aad.gov.au/aadc/portal/download_file.cfm?file_id=1272" \
      "$cache_root/raw/aadc-electricity-monthly-candidate.bin" "CC-BY-4.0; verify asset mapping before use"
    ;;
  fetch-url)
    [[ "$#" -eq 5 ]] || { usage >&2; exit 2; }
    id="$2"; url="$3"; file="$4"; license="$5"
    [[ "$url" == https://* ]] || { echo "ERROR: HTTPS only" >&2; exit 2; }
    case "$url" in
      *sciencebase.gov/*|*usgs.gov/*|*aad.gov.au/*) ;;
      *) echo "ERROR: URL must be from an identified USGS or AADC host" >&2; exit 2 ;;
    esac
    [[ "$file" == /* ]] || file="$cache_root/raw/$file"
    fetch_one "$id" "$url" "$file" "$license"
    ;;
  *) usage >&2; exit 2 ;;
esac

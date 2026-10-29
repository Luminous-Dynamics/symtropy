#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 exact-output.stl output-manifest.json" >&2
  exit 2
fi

stl_path=$1
manifest_path=$2

[[ -f "$stl_path" ]] || { echo "STL file not found: $stl_path" >&2; exit 2; }
[[ -f "$manifest_path" ]] || { echo "manifest file not found: $manifest_path" >&2; exit 2; }

jq -e --arg path "$stl_path" '
  .manifest_schema == "civil-aero-coupon/0.1.0" and
  .artifact_scope == "non_flight_critical_ground_test_coupon" and
  .output_file == $path and
  .output_sha256 == null and
  .watertight_two_manifold == true and
  .signed_volume_mm3 > 0 and
  .manufacturing_release == "not_authorized" and
  .flight_use == "prohibited" and
  .airworthiness_or_structural_claim == false
' "$manifest_path" >/dev/null || {
  echo "manifest rejected: it is not a fresh, matching, closed-volume non-flight coupon" >&2
  exit 2
}

digest=$(sha256sum -- "$stl_path" | awk '{print $1}')
[[ "$digest" =~ ^[0-9a-f]{64}$ ]] || {
  echo "could not compute a lowercase SHA-256 digest for the exact STL bytes" >&2
  exit 2
}

tmp_path=$(mktemp "$manifest_path.tmp.XXXXXX")
cleanup() {
  rm -f -- "$tmp_path"
}
trap cleanup EXIT

jq --arg sha "$digest" \
  '.output_sha256 = $sha | .output_sha256_status = "computed_from_exact_stl_bytes"' \
  "$manifest_path" > "$tmp_path"

mv -- "$tmp_path" "$manifest_path"
trap - EXIT

jq -e '
  (.output_sha256 | type == "string" and test("^[0-9a-f]{64}$")) and
  .output_sha256_status == "computed_from_exact_stl_bytes"
' "$manifest_path" >/dev/null || {
  echo "finalized manifest failed its digest-shape check" >&2
  exit 2
}

sha256sum -- "$stl_path" "$manifest_path"

#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Hash-first registration for external lunar granular-flow source artifacts.
# It never extracts or modifies source bytes and never marks data training-ready.

set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"

usage() {
  cat >&2 <<'USAGE'
Usage:
  register-lunar-granular-flow-source.sh SOURCE_ID FILE_ID FILE_PATH

The source/file identity and expected MD5 must already exist in
tools/lunar-granular-flow-benchmark/data-sources.json.
USAGE
  exit 2
}

fail() {
  printf 'source artifact registration: FAIL: %s\n' "$*" >&2
  exit 1
}

[[ "$#" -eq 3 ]] || usage
source_id="$1"
file_id="$2"
file_path="$3"
registry="tools/lunar-granular-flow-benchmark/data-sources.json"

command -v jq >/dev/null 2>&1 || fail "jq is required"
command -v md5sum >/dev/null 2>&1 || fail "md5sum is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"
command -v realpath >/dev/null 2>&1 || fail "realpath is required"

[[ -f "$file_path" && ! -L "$file_path" ]] || fail "source path must be a regular non-symlink file"
canonical_path="$(realpath -e -- "$file_path")"
filename="$(basename -- "$canonical_path")"

entry="$(jq -ce --arg source "$source_id" --arg file "$file_id" '
  [.sources[] | select(.source_id == $source and .access == "public")
   | .files[] | select(.file_id == $file and .status == "not_downloaded")]
  | if length == 1 then .[0] else error("source/file identity is missing, ambiguous or not public") end
' "$registry")" || fail "source/file identity is not an eligible public registry entry"
expected="$(jq -r '.md5' <<< "$entry")"
declared_filename="$(jq -r '.filename' <<< "$entry")"

[[ "$filename" == "$declared_filename" ]] || fail "filename does not match registry: expected $declared_filename, got $filename"
actual_md5=""
publisher_md5_status="not_provided"
if [[ "$expected" != "null" ]]; then
  actual_md5="$(md5sum -- "$canonical_path" | awk '{print $1}')"
  [[ "$actual_md5" == "$expected" ]] || fail "publisher MD5 mismatch: expected $expected, got $actual_md5"
  publisher_md5_status="verified"
elif [[ "$filename" == *.zip ]]; then
  command -v unzip >/dev/null 2>&1 || fail "unzip is required to validate ZIP structure when no publisher checksum is provided"
  unzip -tqq -- "$canonical_path" || fail "ZIP structure validation failed"
fi

actual_sha256="$(sha256sum -- "$canonical_path" | awk '{print $1}')"
bytes="$(stat -c '%s' -- "$canonical_path")"
retrieved_at="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"

jq -n \
  --arg source_id "$source_id" \
  --arg file_id "$file_id" \
  --arg filename "$filename" \
  --arg path "$canonical_path" \
  --arg md5 "$actual_md5" \
  --arg publisher_md5_status "$publisher_md5_status" \
  --arg sha256 "$actual_sha256" \
  --arg retrieved_at "$retrieved_at" \
  --argjson bytes "$bytes" '
  {
    receipt_class:"external_source_artifact_integrity_receipt",
    evidence_class:"source_bytes_hash_verified_pending_review",
    source_id:$source_id,
    file_id:$file_id,
    filename:$filename,
    original_path:$path,
    bytes:$bytes,
    publisher_md5:(if $md5 == "" then null else $md5 end),
    publisher_md5_status:$publisher_md5_status,
    integrity_basis:(if $publisher_md5_status == "verified" then "publisher_md5_and_local_sha256" else "local_sha256_and_archive_structure_only" end),
    sha256:$sha256,
    retrieved_at_utc:$retrieved_at,
    admission_status:"pending_schema_license_and_provenance_review",
    training_eligible:false,
    extracted:false,
    source_bytes_modified:false,
    claim_ceiling:"Byte integrity only. This receipt does not establish licence permission, semantic validity, experiment provenance, calibration validity, physical accuracy or benchmark eligibility."
  }
'

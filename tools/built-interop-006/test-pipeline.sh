#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

schema_gate_root="${BUILT_INTEROP_005_ROOT:-$repo_root/tools/built-interop-005}"
schema_gate_manifest="$schema_gate_root/Cargo.toml"
fixture_ifc="$repo_root/tools/built-interop-006/fixtures/panel.ifc"
fixture_ids="$repo_root/tools/built-interop-006/fixtures/panel.ids"
schema_gate_json="$tmp/ifc-schema-gate.json"
assembled_json="$tmp/assembled-report.json"

# The validator may return 0 (PASS) or 2 (UNRESOLVED) for this fixture, but a
# schema error (1) must fail the integration contract rather than be masked.
set +e
cargo run --locked --manifest-path "$schema_gate_manifest" -- \
  "$fixture_ifc" > "$schema_gate_json"
schema_exit=$?
set -e
if [[ "$schema_exit" -ne 0 && "$schema_exit" -ne 2 ]]; then
  echo "SCHEMA_GATE_UNEXPECTED_EXIT: $schema_exit" >&2
  exit 1
fi

jq -e '
  .benchmark_id == "BUILT-INTEROP-005" and
  .gate.gate_id == "ifc_schema_conformance" and
  .gate.mandatory == true and
  (.gate.status == "PASS" or .gate.status == "UNRESOLVED") and
  (.artifact.sha256 | test("^[a-f0-9]{64}$")) and
  (.gate.evidence | length > 0) and
  (.gate.evidence[0].sha256 | test("^[a-f0-9]{64}$"))
' "$schema_gate_json" >/dev/null || {
  echo "SCHEMA_GATE_OUTPUT_CONTRACT_INVALID" >&2
  exit 1
}

run_id="${GITHUB_RUN_ID:-local}-${GITHUB_RUN_ATTEMPT:-1}"
reported_at="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
source_commit_sha="${GITHUB_SHA:-0000000000000000000000000000000000000000}"

# Only the real schema gate is supplied. The other seven gates must be explicit
# NOT_RUN, making a successful integration of these tools UNQUALIFIED by design.
set +e
cargo run --locked --manifest-path "$repo_root/tools/built-interop-006/Cargo.toml" -- \
  --source-ifc "$fixture_ifc" \
  --ids "$fixture_ids" \
  --target-ifc "$fixture_ifc" \
  --ifc-schema IFC4 \
  --supported-subset "synthetic-ifc4-ids1.0-panel-v0.1" \
  --run-id "$run_id" \
  --reported-at "$reported_at" \
  --source-commit-sha "$source_commit_sha" \
  --evaluator-name "built-interop-006-integration" \
  --gate-json "$schema_gate_json" \
  --output "$assembled_json" > "$tmp/stdout.json"
assembler_exit=$?
set -e
if [[ "$assembler_exit" -ne 2 ]]; then
  echo "ASSEMBLER_MUST_REMAIN_UNQUALIFIED_WITH_SEVEN_MISSING_GATES: exit=$assembler_exit" >&2
  exit 1
fi

jq -e '
  .benchmark_id == "BUILT-INTEROP-004" and
  .overall_decision == "UNQUALIFIED" and
  (.artifacts | length == 3) and
  (.gates | length == 8) and
  ([.gates[].gate_id] | unique | length == 8) and
  ([.gates[] | select(.gate_id == "ifc_schema_conformance" and (.status == "PASS" or .status == "UNRESOLVED"))] | length == 1) and
  ([.gates[] | select(.gate_id != "ifc_schema_conformance" and .status == "NOT_RUN" and .evidence == [] and (has("tool") | not))] | length == 7) and
  all(.artifacts[]; (.sha256 | test("^[a-f0-9]{64}$")) and (.byte_length | type == "number"))
' "$assembled_json" >/dev/null || {
  echo "ASSEMBLED_REPORT_INTEGRATION_CONTRACT_INVALID" >&2
  exit 1
}

printf 'BUILT_INTEROP_005_TO_006_INTEGRATION_OK: schema_gate=%s missing_gates=7 overall=UNQUALIFIED\n' \
  "$(jq -r '.gate.status' "$schema_gate_json")"

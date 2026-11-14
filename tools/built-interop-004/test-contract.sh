#!/usr/bin/env bash
set -euo pipefail

here="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
validator="$here/validate-report.sh"
schema="$here/report-v1.schema.json"
command -v jq >/dev/null 2>&1 || { echo "MISSING_TOOL: jq" >&2; exit 2; }
jq empty "$schema" >/dev/null || { echo "REPORT_SCHEMA_JSON_INVALID" >&2; exit 1; }
jq -e '
  ."$schema" == "https://json-schema.org/draft/2020-12/schema" and
  .additionalProperties == false and
  .properties.schema_version.const == "1.0.0" and
  .properties.benchmark_id.const == "BUILT-INTEROP-004" and
  .properties.evaluation_profile.properties.ids_version.const == "1.0" and
  .properties.gates.minItems == 8 and .properties.gates.maxItems == 8 and
  any(.properties.gates.items.allOf[];
    .if.properties.status.const == "NOT_RUN" and
    .then.properties.evidence.maxItems == 0 and
    .then.not.required == ["tool"]
  ) and
  ([.properties.artifacts.items.properties.role.enum[]] | sort) ==
    (["source_ifc", "requirements_ids", "target_ifc"] | sort) and
  ([.properties.gates.items.properties.gate_id.enum[]] | sort) == ([
    "artifact_integrity", "step_syntax", "ifc_schema_conformance",
    "ids_xsd_validity", "ids_semantic_validity", "ids_model_conformance",
    "exchange_semantic_preservation", "engineering_release_qualification"
  ] | sort) and
  ([.properties.gates.items.properties.status.enum[]] | sort) ==
    (["PASS", "FAIL", "UNRESOLVED", "NOT_RUN"] | sort) and
  ([.properties.overall_decision.enum[]] | sort) ==
    (["QUALIFIED", "BLOCKED", "UNQUALIFIED"] | sort)
' "$schema" >/dev/null || { echo "REPORT_SCHEMA_CONTRACT_MISMATCH" >&2; exit 1; }
bash -n "$validator"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# All evidence is intentionally synthetic: these are contract controls, not IFC claims.
jq -n '
  ["artifact_integrity", "step_syntax", "ifc_schema_conformance",
   "ids_xsd_validity", "ids_semantic_validity", "ids_model_conformance",
   "exchange_semantic_preservation", "engineering_release_qualification"] as $ids |
  {
    schema_version: "1.0.0",
    benchmark_id: "BUILT-INTEROP-004",
    run_id: "synthetic-contract-control-001",
    reported_at: "2026-10-18T00:00:00Z",
    evaluation_profile: {
      ifc_schema: "IFC4",
      ids_version: "1.0",
      supported_subset: "synthetic contract self-test; no building qualification"
    },
    evaluator: {
      name: "built-interop-004-contract-test",
      version: "0.1.0",
      source_commit_sha: ("f" * 40)
    },
    artifacts: [
      {role:"source_ifc", sha256:(("a" * 63) + "1"), byte_length:10, media_type:"model/ifc", locator:"fixture://source.ifc"},
      {role:"requirements_ids", sha256:(("a" * 63) + "2"), byte_length:11, media_type:"application/ids+xml", locator:"fixture://requirements.ids"},
      {role:"target_ifc", sha256:(("a" * 63) + "3"), byte_length:12, media_type:"model/ifc", locator:"fixture://target.ifc"}
    ],
    gates: ($ids | to_entries | map(
      . as $gate |
      {
        gate_id:$gate.value,
        mandatory:true,
        status:"PASS",
        claim_scope:"synthetic only",
        limitations:["synthetic evidence only; do not infer real IFC or engineering validity"],
        tool:{name:"contract-fixture-producer", version:"0.1.0", config_sha256:("a" * 64)},
        evidence:[{
          evidence_id:("evidence-" + ($gate.key | tostring)),
          kind:"mutation_test",
          sha256:(("b" * 63) + ($gate.key | tostring)),
          locator:("fixture://evidence/" + ($gate.key | tostring))
        }]
      }
    )),
    overall_decision:"QUALIFIED",
    scope_statement:"Synthetic report used only to exercise the contract checker.",
    limitations:["No IFC, IDS, CAD, geometry, code or engineering validation was performed."]
  }
' > "$tmp/qualified.json"

bash "$validator" "$tmp/qualified.json" >/dev/null

# A valid but incomplete run remains UNQUALIFIED; a real gate failure is BLOCKED.
jq '
  .gates |= map(if .gate_id == "ids_semantic_validity" then .status = "UNRESOLVED" else . end) |
  .overall_decision = "UNQUALIFIED"
' "$tmp/qualified.json" > "$tmp/unqualified.json"
bash "$validator" "$tmp/unqualified.json" >/dev/null

jq '
  .gates |= map(if .gate_id == "ifc_schema_conformance" then .status = "FAIL" else . end) |
  .overall_decision = "BLOCKED"
' "$tmp/qualified.json" > "$tmp/blocked.json"
bash "$validator" "$tmp/blocked.json" >/dev/null

expect_reject() {
  local label="$1" path="$2"
  if bash "$validator" "$path" >/dev/null 2>&1; then
    echo "NEGATIVE_CONTROL_ACCEPTED: $label" >&2
    exit 1
  fi
  printf 'NEGATIVE_CONTROL_REJECTED: %s\n' "$label"
}

jq '
  .gates |= map(if .gate_id == "step_syntax" then .status = "NOT_RUN" | .evidence = [] | del(.tool) else . end)
' "$tmp/qualified.json" > "$tmp/qualified-with-not-run.json"
expect_reject "NOT_RUN cannot yield QUALIFIED" "$tmp/qualified-with-not-run.json"

jq '.gates |= map(select(.gate_id != "ids_xsd_validity")) | .overall_decision = "UNQUALIFIED"' \
  "$tmp/qualified.json" > "$tmp/missing-gate.json"
expect_reject "missing mandatory gate" "$tmp/missing-gate.json"

jq '
  .gates |= map(if .gate_id == "step_syntax" then .gate_id = "artifact_integrity" else . end) |
  .overall_decision = "UNQUALIFIED"
' "$tmp/qualified.json" > "$tmp/duplicate-gate.json"
expect_reject "duplicate gate identity" "$tmp/duplicate-gate.json"

jq '
  .gates |= map(if .gate_id == "ids_model_conformance" then .evidence = [] else . end)
' "$tmp/qualified.json" > "$tmp/pass-without-evidence.json"
expect_reject "PASS without evidence" "$tmp/pass-without-evidence.json"

jq '
  .gates |= map(if .gate_id == "step_syntax" then .status = "NOT_RUN" | .evidence = [] | del(.tool) else . end) |
  .overall_decision = "BLOCKED"
' "$tmp/qualified.json" > "$tmp/decision-mismatch.json"
expect_reject "decision mismatch" "$tmp/decision-mismatch.json"

jq '.artifacts[0].sha256 = "deadbeef"' "$tmp/qualified.json" > "$tmp/malformed-artifact-hash.json"
expect_reject "malformed artifact hash" "$tmp/malformed-artifact-hash.json"

jq '
  .gates |= map(if .gate_id == "step_syntax" then .status = "NOT_RUN" else . end) |
  .overall_decision = "UNQUALIFIED"
' "$tmp/qualified.json" > "$tmp/not-run-with-evidence.json"
expect_reject "NOT_RUN with tool/evidence attached" "$tmp/not-run-with-evidence.json"

jq '.unexpected_claim = true' "$tmp/qualified.json" > "$tmp/unknown-field.json"
expect_reject "unknown top-level field" "$tmp/unknown-field.json"

jq '.reported_at = "2026-99-99 not-a-timestamp"' "$tmp/qualified.json" > "$tmp/malformed-timestamp.json"
expect_reject "malformed report timestamp" "$tmp/malformed-timestamp.json"

echo "BUILT_INTEROP_004_CONTRACT_TESTS_OK: accepted_valid=3 rejected_negative_controls=9"

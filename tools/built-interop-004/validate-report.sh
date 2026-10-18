#!/usr/bin/env bash
set -euo pipefail

report="${1:?Usage: validate-report.sh report.json}"
command -v jq >/dev/null 2>&1 || { echo "MISSING_TOOL: jq" >&2; exit 2; }
[[ -f "$report" ]] || { echo "MISSING_REPORT: $report" >&2; exit 2; }
jq empty "$report" >/dev/null 2>&1 || { echo "REPORT_JSON_INVALID" >&2; exit 1; }

require_contract() {
  local code="$1"
  local expression="$2"
  if ! jq -e "$expression" "$report" >/dev/null; then
    echo "$code" >&2
    exit 1
  fi
}

require_contract REPORT_ENVELOPE_INVALID '
  type == "object" and
  ([keys[]] | sort) == ([
    "schema_version", "benchmark_id", "run_id", "reported_at",
    "evaluation_profile", "evaluator", "artifacts", "gates",
    "overall_decision", "scope_statement", "limitations"
  ] | sort) and
  .schema_version == "1.0.0" and
  .benchmark_id == "BUILT-INTEROP-004" and
  (.run_id | type == "string" and length > 0) and
  (.reported_at | type == "string" and test("^[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01])T([01][0-9]|2[0-3]):[0-5][0-9]:([0-5][0-9]|60)(\\.[0-9]+)?(Z|[+-]([01][0-9]|2[0-3]):[0-5][0-9])$")) and
  (.scope_statement | type == "string" and length > 0) and
  (.limitations | type == "array" and length > 0 and all(.[]; type == "string" and length > 0)) and
  (.evaluation_profile | type == "object" and
    ([keys[]] | sort) == (["ifc_schema", "ids_version", "supported_subset"] | sort) and
    (.ifc_schema | type == "string" and length > 0) and
    .ids_version == "1.0" and
    (.supported_subset | type == "string" and length > 0)) and
  (.evaluator | type == "object" and
    ([keys[]] | sort) == (["name", "version", "source_commit_sha"] | sort) and
    (.name | type == "string" and length > 0) and
    (.version | type == "string" and length > 0) and
    (.source_commit_sha | test("^[a-f0-9]{40}$"))
  )
'

require_contract ARTIFACT_SET_INVALID '
  (.artifacts | type == "array" and length == 3) and
  ([.artifacts[].role] | sort) == (["source_ifc", "requirements_ids", "target_ifc"] | sort) and
  all(.artifacts[];
    type == "object" and
    ([keys[]] | sort) == (["role", "sha256", "byte_length", "media_type", "locator"] | sort) and
    (.sha256 | type == "string" and test("^[a-f0-9]{64}$")) and
    (.byte_length | type == "number" and . >= 0 and floor == .) and
    (.media_type | type == "string" and length > 0) and
    (.locator | type == "string" and length > 0)
  )
'

require_contract GATE_SET_INVALID '
  (.gates | type == "array" and length == 8) and
  ([.gates[].gate_id] | sort) == ([
    "artifact_integrity", "step_syntax", "ifc_schema_conformance",
    "ids_xsd_validity", "ids_semantic_validity", "ids_model_conformance",
    "exchange_semantic_preservation", "engineering_release_qualification"
  ] | sort) and
  all(.gates[];
    type == "object" and
    (([keys[]] | sort) == (["gate_id", "mandatory", "status", "claim_scope", "limitations", "evidence"] | sort) or
     ([keys[]] | sort) == (["gate_id", "mandatory", "status", "claim_scope", "limitations", "evidence", "tool"] | sort)) and
    .mandatory == true and
    (.status == "PASS" or .status == "FAIL" or .status == "UNRESOLVED" or .status == "NOT_RUN") and
    (.claim_scope | type == "string" and length > 0) and
    (.limitations | type == "array" and length > 0 and all(.[]; type == "string" and length > 0)) and
    (.evidence | type == "array") and
    (if has("tool") then
      (.tool | type == "object" and
        ([keys[]] | sort) == (["name", "version", "config_sha256"] | sort) and
        (.name | type == "string" and length > 0) and
        (.version | type == "string" and length > 0) and
        (.config_sha256 | type == "string" and test("^[a-f0-9]{64}$")))
     else true end) and
    all(.evidence[];
      type == "object" and
      ([keys[]] | sort) == (["evidence_id", "kind", "sha256", "locator"] | sort) and
      (.evidence_id | type == "string" and length > 0) and
      (.kind == "tool_output" or .kind == "validator_report" or .kind == "mutation_test" or .kind == "external_attestation" or .kind == "artifact_digest" or .kind == "review_record") and
      (.sha256 | type == "string" and test("^[a-f0-9]{64}$")) and
      (.locator | type == "string" and length > 0)
    )
  )
'

require_contract GATE_EXECUTION_EVIDENCE_INVALID '
  all(.gates[];
    if .status == "PASS" or .status == "FAIL" then
      has("tool") and (.evidence | length > 0)
    elif .status == "NOT_RUN" then
      (has("tool") | not) and (.evidence | length == 0)
    else true end
  ) and
  ([.gates[].evidence[].evidence_id] | unique | length) ==
    ([.gates[].evidence[].evidence_id] | length)
'

require_contract OVERALL_DECISION_MISMATCH '
  .overall_decision ==
    (if any(.gates[]; .status == "FAIL") then "BLOCKED"
     elif all(.gates[]; .status == "PASS") then "QUALIFIED"
     else "UNQUALIFIED" end)
'

printf 'GATE_REPORT_CONTRACT_VALID: benchmark=%s run=%s decision=%s\n' \
  "$(jq -r '.benchmark_id' "$report")" \
  "$(jq -r '.run_id' "$report")" \
  "$(jq -r '.overall_decision' "$report")"
printf 'NOTE: validates report structure and decision consistency only; evidence content and tool soundness are not verified here.\n'

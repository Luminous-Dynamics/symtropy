#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CATALOG="$ROOT/research/polar-infrastructure/stressors-v0.json"
SELF_TEST=0
if [[ "$#" -gt 0 ]]; then
  if [[ "$1" == "--self-test" ]]; then SELF_TEST=1; else CATALOG="$1"; fi
fi
command -v jq >/dev/null 2>&1 || { echo "ERROR: jq is required" >&2; exit 2; }
[[ -r "$CATALOG" ]] || { echo "ERROR: cannot read catalog: $CATALOG" >&2; exit 2; }

# Structural smoke-check only; full JSON Schema validation is a separate step.
jq -e '
 .schema_version == "0.1.0"
 and .artifact_kind == "polar-infrastructure-stressor-catalog"
 and .status == "proposed"
 and .evidence_claim_ceiling == "synthetic_only"
 and (.contexts | type == "array" and length >= 3)
 and (.reference_basis | type == "array" and length >= 3)
 and (.benchmark_rules | type == "array" and length >= 5)
 and (.scenarios | type == "array" and length >= 10)
 and (([.scenarios[].id] | unique | length) == (.scenarios | length))
 and all(.scenarios[];
   (.id | type == "string" and test("^POLAR-[0-9]{3}$"))
   and (.applicable_contexts | type == "array" and length >= 1)
   and (.trigger | type == "object")
   and (.required_inputs | type == "array" and length >= 1)
   and (.outputs | type == "array" and length >= 1)
   and (.hard_gates | type == "array" and length >= 1)
   and (.assumptions | type == "array" and length >= 1)
   and (.test_status == "not_run")
   and (has("data_kind") | not)
 )
' "$CATALOG" >/dev/null || { echo "ERROR: catalog violates structural contract" >&2; exit 1; }

if [[ "$SELF_TEST" -eq 0 ]]; then
  echo "catalog smoke-check passed; physical behavior and safety have not been validated"
  exit 0
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
check_rejected() {
  local name="$1" expression="$2"
  jq "$expression" "$ROOT/research/polar-infrastructure/stressors-v0.json" > "$TMP/mutated.json"
  if "$0" "$TMP/mutated.json" >/dev/null 2>&1; then
    echo "ERROR: negative control accepted: $name" >&2
    exit 1
  fi
  echo "rejected: $name"
}
check_rejected "synthetic-to-domain-validated promotion" '.evidence_claim_ceiling = "domain_validated"'
check_rejected "scenario marked passed without receipt" '.scenarios[0].test_status = "pass"'
check_rejected "duplicate scenario ID" '.scenarios[1].id = .scenarios[0].id'
check_rejected "empty safety gates" '.scenarios[0].hard_gates = []'
check_rejected "missing context" '.scenarios[0].applicable_contexts = []'
check_rejected "insufficient scenarios" '.scenarios = .scenarios[0:4]'
check_rejected "data-kind overclaim" '.scenarios[0].data_kind = "measured"'
echo "7 negative controls rejected; this is not physical or operational validation"

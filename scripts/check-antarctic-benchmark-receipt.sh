#!/usr/bin/env bash
# Copyright (C) 2026 Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Structural smoke-test for the versioned run receipt and its synthetic example.
# This does not replace a full JSON Schema validator or adjudicate physical truth.

set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
schema="${1:-$repo_root/research/antarctic-energy-resilience/benchmark-run-v0.schema.json}"
example="${2:-$repo_root/research/antarctic-energy-resilience/benchmark-run-v0.synthetic.json}"

command -v jq >/dev/null 2>&1 || { echo "ERROR: jq is required" >&2; exit 2; }
test -f "$schema" && test -f "$example" || { echo "ERROR: missing schema/example file" >&2; exit 2; }

jq -e '
  . as $root
  | $root["$schema"] == "https://json-schema.org/draft/2020-12/schema"
  and $root.title == "ANTARCTIC-ENERGY-001 Benchmark Run Receipt v0"
  and $root.type == "object"
  and $root.additionalProperties == false
  and all($root.required[]; type == "string")
  and all(["schema_version","run_id","status","evidence_tier","case_manifest",
           "source_artifacts","split_plan","methods","budget","hard_gates",
           "metrics","validation","claim_ceiling"][]; . as $k | ($root.properties | has($k)))
  and $root["$defs"].gate.enum == ["pass","fail","unknown","not_applicable"]
  and $root.properties.validation.properties.ci_status.enum
      == ["not_run","queued","skipped","pass_exact_head","fail","stale_or_unknown"]
' "$schema" >/dev/null

audit_example() {
  local candidate="$1"
  jq -e '
    .schema_version == "antarctic-benchmark-run-v0.1"
    and .run_id == "synthetic-contract-plan-v0"
    and .status == "planned"
    and .evidence_tier == "S0_synthetic"
    and .case_manifest.physical_data == false
    and .case_manifest.case_sha256 == null
    and (.source_artifacts | length) == 0
    and .split_plan.method == "synthetic_only"
    and .split_plan.locked_before_tuning == true
    and (.methods | length) >= 1
    and (.budget.primary_budget_unit == "synthetic_fixture_only")
    and .hard_gates.electric_balance == "unknown"
    and .hard_gates.thermal_balance == "unknown"
    and .validation.source_validation == "not_started"
    and .validation.independent_audit == "not_started"
    and .validation.ci_status == "not_run"
    and .claim_ceiling == "software_accounting_only"
  ' "$candidate" >/dev/null
}

audit_example "$example"
echo "PASS: schema and synthetic planned-run example carry evidence, split, budget, hard-gate, validation and claim-ceiling fields."

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT
jq '.case_manifest.physical_data = true' "$example" >"$tmp"
if audit_example "$tmp" >/dev/null 2>&1; then
  echo "ERROR: synthetic example accepted with physical_data=true" >&2
  exit 1
fi
jq '.claim_ceiling = "operationally_validated_within_scope"' "$example" >"$tmp"
if audit_example "$tmp" >/dev/null 2>&1; then
  echo "ERROR: planned synthetic example promoted to an operational claim" >&2
  exit 1
fi
echo "PASS: negative controls reject synthetic-to-physical and synthetic-to-operational claim promotion."
sha256sum "$schema" "$example"

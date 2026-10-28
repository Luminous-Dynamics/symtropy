#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage:
  bash evaluate_environment_coverage.sh ASSESSMENT.json [REPORT.json]

Reads one declared source/variable/window coverage assessment. An absent
minimum_coverage_fraction blocks metric scoring; missing rows are never zeros.
Exit codes: 0=coverage gate passed, 2=invalid input, 3=threshold unset,
4=insufficient coverage. A passed coverage gate does not validate source origin,
license, scientific values, model correctness, or physical suitability.
USAGE
  exit 64
}

[[ $# -ge 1 && $# -le 2 ]] || usage
INPUT=$1
OUTPUT=${2:-/dev/stdout}

[[ -f "$INPUT" && -r "$INPUT" ]] || { echo "assessment file must be readable" >&2; exit 2; }
command -v jq >/dev/null || { echo "jq is required" >&2; exit 69; }
if ! jq empty "$INPUT" >/dev/null 2>&1; then
  echo "assessment is not valid JSON" >&2
  exit 2
fi

VALID=$(jq -e '
  type == "object" and
  .schema_version == "0.1.0" and
  .artifact_kind == "symtropy-environment-coverage-assessment" and
  (.assessment_id | type == "string" and length > 0) and
  (.source_id | type == "string" and length > 0) and
  (.variable_id | type == "string" and length > 0) and
  (.window_id | type == "string" and length > 0) and
  (.expected_sample_count | type == "number" and floor == . and . >= 1) and
  (.observed_sample_count | type == "number" and floor == . and . >= 0) and
  (.observed_sample_count <= .expected_sample_count) and
  (.minimum_coverage_fraction == null or
    (.minimum_coverage_fraction | type == "number" and . >= 0 and . <= 1))
' "$INPUT" 2>/dev/null || true)

if [[ "$VALID" != "true" ]]; then
  jq -n --arg input "$(basename "$INPUT")" '{
    report_kind:"symtropy-environment-coverage-report",
    decision:"invalid_input",
    score_eligible:false,
    validation_scope:"coverage_contract_only_not_source_or_physical_validation",
    reason:"required fields, count relationships, or threshold value are invalid",
    input_filename:$input
  }' > "$OUTPUT"
  echo "invalid environment coverage assessment; see report" >&2
  exit 2
fi

REPORT=$(jq '
  . as $a |
  ($a.observed_sample_count / $a.expected_sample_count) as $coverage |
  {
    report_kind:"symtropy-environment-coverage-report",
    schema_version:"0.1.0",
    assessment_id:$a.assessment_id,
    source_id:$a.source_id,
    variable_id:$a.variable_id,
    window_id:$a.window_id,
    expected_sample_count:$a.expected_sample_count,
    observed_sample_count:$a.observed_sample_count,
    missing_sample_count:($a.expected_sample_count - $a.observed_sample_count),
    coverage_fraction:$coverage,
    minimum_coverage_fraction:$a.minimum_coverage_fraction,
    decision:(if $a.minimum_coverage_fraction == null then "blocked_threshold_unset"
      elif $coverage < $a.minimum_coverage_fraction then "insufficient_coverage"
      else "coverage_gate_passed" end),
    score_eligible:($a.minimum_coverage_fraction != null and $coverage >= $a.minimum_coverage_fraction),
    missing_values_are_zero:false,
    validation_scope:"coverage_contract_only_not_source_origin_license_or_physical_validation"
  }
' "$INPUT")
printf '%s\n' "$REPORT" > "$OUTPUT"
DECISION=$(jq -r '.decision' <<< "$REPORT")
case "$DECISION" in
  coverage_gate_passed) exit 0 ;;
  blocked_threshold_unset) echo "coverage threshold has not been preregistered; metric scoring blocked" >&2; exit 3 ;;
  insufficient_coverage) echo "observed coverage is below the preregistered threshold; metric scoring blocked" >&2; exit 4 ;;
  *) echo "unexpected coverage decision: $DECISION" >&2; exit 2 ;;
esac

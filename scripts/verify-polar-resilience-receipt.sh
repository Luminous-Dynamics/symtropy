#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'verify-polar-resilience-receipt: %s\n' "$*" >&2
  return 1
}

verify_bundle() {
  local dir="$1"
  [[ -d "$dir" ]] || fail "artifact directory does not exist: $dir"
  (
    cd "$dir"
    for required in run-receipt.json run-receipt.sha256 SHA256SUMS TESTED_COMMIT; do
      [[ -f "$required" ]] || { printf 'missing required file: %s\n' "$required" >&2; exit 1; }
    done

    # These checks establish integrity relative to the included manifests. The
    # receipt checksum must itself be compared with a trusted copy for authenticity.
    sha256sum -c SHA256SUMS || { printf 'SHA256SUMS verification failed\n' >&2; exit 1; }
    sha256sum -c run-receipt.sha256 || { printf 'run-receipt.sha256 verification failed\n' >&2; exit 1; }

    jq -e '
      .receipt_schema_version == "0.2.0" and
      .evidence_class == "synthetic_only" and
      .receipt_status == "checks_completed" and
      (.tested_commit | type == "string" and test("^[0-9a-f]{40}$")) and
      (.tested_repository | type == "string" and length > 0) and
      (.workflow_run_id | tostring | test("^[0-9]+$")) and
      (.workflow_attempt | tostring | test("^[0-9]+$")) and
      .tested_commit_path == "TESTED_COMMIT" and
      .verifier_path == "verify-polar-resilience-receipt.sh" and
      .input_schema_versions == {score:"0.1.0", ensemble:"0.1.0", robust_set:"0.1.0"} and
      .report_schema_versions == {score:"0.2.0", ensemble:"0.4.0", robust_set:"0.4.0"} and
      .input_paths == [
        "inputs/Cargo.toml",
        "inputs/Cargo.lock",
        "inputs/rust-toolchain.toml",
        "inputs/source/lib.rs",
        "inputs/source/main.rs",
        "inputs/source/bin/resilience_ensemble.rs",
        "inputs/source/bin/robust_set.rs",
        "inputs/polar-resilience-scorer.yml",
        "inputs/toy-episode.json",
        "inputs/toy-ensemble.json",
        "inputs/toy-robust-set.json"
      ] and
      .report_paths == [
        "reports/score-report.json",
        "reports/ensemble-report.json",
        "reports/robust-set-report.json"
      ] and
      .checksums_file == "SHA256SUMS"
    ' run-receipt.json >/dev/null || { printf 'receipt contract is invalid\n' >&2; exit 1; }

    local expected_commit
    expected_commit="$(jq -r '.tested_commit' run-receipt.json)"
    [[ "$(tr -d '\r\n' < TESTED_COMMIT)" == "$expected_commit" ]] || {
      printf 'TESTED_COMMIT does not match receipt\n' >&2
      exit 1
    }

    # Enforce checksum coverage too: sha256sum -c alone does not detect a
    # manifest with required files silently omitted.
    local path
    while IFS= read -r path; do
      grep -Fq "  $path" SHA256SUMS || {
        printf 'SHA256SUMS omits required path: %s\n' "$path" >&2
        exit 1
      }
    done < <(jq -r '.input_paths[], .report_paths[], .tested_commit_path, .verifier_path' run-receipt.json)

    jq -e '
      .report_kind == "polar-resilience-score-report" and
      .schema_version == "0.2.0" and
      .evidence_class == "synthetic_only" and
      .result_scope == "metric_calculation_only_not_physical_validation" and
      (.episode_duration_hours | type == "number" and . > 0) and
      (.results | length == 4) and
      ([.results[] | select(.policy_id == "defective_low_fuel_control" and .gate_pass == false)] | length == 1)
    ' reports/score-report.json >/dev/null || { printf 'score report contract is invalid\n' >&2; exit 1; }

    jq -e '
      .report_kind == "polar-resilience-ensemble-report" and
      .schema_version == "0.4.0" and
      .evidence_class == "synthetic_only" and
      .result_scope == "weighted_synthetic_scenario_aggregation_not_physical_validation" and
      (.scenario_count | type == "number" and . > 0) and
      ([.results[] | select(.policy_id == "candidate_full_service" and (.service_results | length == 3))] | length == 1)
    ' reports/ensemble-report.json >/dev/null || { printf 'ensemble report contract is invalid\n' >&2; exit 1; }

    jq -e '
      .report_kind == "polar-resilience-robust-set-report" and
      .schema_version == "0.4.0" and
      .evidence_class == "synthetic_only" and
      .result_scope == "unweighted_enumerated_stress_set_not_probability_estimate" and
      .pareto_comparison_status == "complete" and
      (.scenario_count | type == "number" and . > 0) and
      ([.policy_results[] | select(.policy_id == "candidate_full_service" and (.service_results | length == 3))] | length == 1)
    ' reports/robust-set-report.json >/dev/null || { printf 'robust-set report contract is invalid\n' >&2; exit 1; }

    printf 'Verified integrity and synthetic report contracts for commit %s\n' "$expected_commit"
  )
}

make_test_bundle() {
  local dir="$1"
  mkdir -p "$dir/inputs/source/bin" "$dir/reports"
  printf 'package = "test"\n' > "$dir/inputs/Cargo.toml"
  printf 'version = 3\n' > "$dir/inputs/Cargo.lock"
  printf 'channel = "stable"\n' > "$dir/inputs/rust-toolchain.toml"
  printf 'pub fn sample() {}\n' > "$dir/inputs/source/lib.rs"
  printf 'fn main() {}\n' > "$dir/inputs/source/main.rs"
  printf 'fn main() {}\n' > "$dir/inputs/source/bin/resilience_ensemble.rs"
  printf 'fn main() {}\n' > "$dir/inputs/source/bin/robust_set.rs"
  printf 'name: test\n' > "$dir/inputs/polar-resilience-scorer.yml"
  printf '{"schema_version":"0.1.0"}\n' > "$dir/inputs/toy-episode.json"
  printf '{"schema_version":"0.1.0"}\n' > "$dir/inputs/toy-ensemble.json"
  printf '{"schema_version":"0.1.0"}\n' > "$dir/inputs/toy-robust-set.json"
  jq -n '{report_kind:"polar-resilience-score-report", schema_version:"0.2.0", evidence_class:"synthetic_only", result_scope:"metric_calculation_only_not_physical_validation", episode_duration_hours:4, results:[{policy_id:"candidate_full_service",gate_pass:true},{policy_id:"incumbent_reference",gate_pass:true},{policy_id:"transparent_rules",gate_pass:true},{policy_id:"defective_low_fuel_control",gate_pass:false}]}' > "$dir/reports/score-report.json"
  jq -n '{report_kind:"polar-resilience-ensemble-report", schema_version:"0.4.0", evidence_class:"synthetic_only", result_scope:"weighted_synthetic_scenario_aggregation_not_physical_validation", scenario_count:2, results:[{policy_id:"candidate_full_service",service_results:[{},{},{}]}]}' > "$dir/reports/ensemble-report.json"
  jq -n '{report_kind:"polar-resilience-robust-set-report", schema_version:"0.4.0", evidence_class:"synthetic_only", result_scope:"unweighted_enumerated_stress_set_not_probability_estimate", pareto_comparison_status:"complete", scenario_count:3, policy_results:[{policy_id:"candidate_full_service",service_results:[{},{},{}]}]}' > "$dir/reports/robust-set-report.json"
  printf '%s\n' '0123456789abcdef0123456789abcdef01234567' > "$dir/TESTED_COMMIT"
  cp "$0" "$dir/verify-polar-resilience-receipt.sh"
  (
    cd "$dir"
    sha256sum \
      inputs/Cargo.toml \
      inputs/Cargo.lock \
      inputs/rust-toolchain.toml \
      inputs/source/lib.rs \
      inputs/source/main.rs \
      inputs/source/bin/resilience_ensemble.rs \
      inputs/source/bin/robust_set.rs \
      inputs/polar-resilience-scorer.yml \
      inputs/toy-episode.json \
      inputs/toy-ensemble.json \
      inputs/toy-robust-set.json \
      reports/score-report.json \
      reports/ensemble-report.json \
      reports/robust-set-report.json \
      TESTED_COMMIT \
      verify-polar-resilience-receipt.sh \
      > SHA256SUMS
    jq -n \
      --arg tested_commit "$(tr -d '\r\n' < TESTED_COMMIT)" \
      '{receipt_schema_version:"0.2.0",evidence_class:"synthetic_only",receipt_status:"checks_completed",tested_commit:$tested_commit,tested_repository:"Luminous-Dynamics/symtropy",workflow_run_id:"12345",workflow_attempt:"1",workflow_event:"pull_request",tested_commit_path:"TESTED_COMMIT",verifier_path:"verify-polar-resilience-receipt.sh",input_schema_versions:{score:"0.1.0",ensemble:"0.1.0",robust_set:"0.1.0"},report_schema_versions:{score:"0.2.0",ensemble:"0.4.0",robust_set:"0.4.0"},input_paths:["inputs/Cargo.toml","inputs/Cargo.lock","inputs/rust-toolchain.toml","inputs/source/lib.rs","inputs/source/main.rs","inputs/source/bin/resilience_ensemble.rs","inputs/source/bin/robust_set.rs","inputs/polar-resilience-scorer.yml","inputs/toy-episode.json","inputs/toy-ensemble.json","inputs/toy-robust-set.json"],report_paths:["reports/score-report.json","reports/ensemble-report.json","reports/robust-set-report.json"],checksums_file:"SHA256SUMS",caveat:"Synthetic scorer contract evidence only; not physical validation or controller certification."}' > run-receipt.json
    sha256sum run-receipt.json > run-receipt.sha256
  )
}

if [[ "${1:-}" == "--self-test" ]]; then
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  make_test_bundle "$tmp/valid"
  verify_bundle "$tmp/valid" >/dev/null

  cp -a "$tmp/valid" "$tmp/tampered-content"
  printf '\n' >> "$tmp/tampered-content/reports/score-report.json"
  if verify_bundle "$tmp/tampered-content" >/dev/null 2>&1; then
    fail "tampered report unexpectedly passed"
  fi

  cp -a "$tmp/valid" "$tmp/omitted-path"
  grep -v 'reports/score-report.json' "$tmp/omitted-path/SHA256SUMS" > "$tmp/omitted-path/SHA256SUMS.new"
  mv "$tmp/omitted-path/SHA256SUMS.new" "$tmp/omitted-path/SHA256SUMS"
  if verify_bundle "$tmp/omitted-path" >/dev/null 2>&1; then
    fail "manifest with omitted report unexpectedly passed"
  fi

  cp -a "$tmp/valid" "$tmp/escalated-claim"
  jq '.evidence_class = "domain_validated"' "$tmp/escalated-claim/reports/score-report.json" > "$tmp/escalated-claim/reports/score-report.json.new"
  mv "$tmp/escalated-claim/reports/score-report.json.new" "$tmp/escalated-claim/reports/score-report.json"
  (
    cd "$tmp/escalated-claim"
    sha256sum \
      inputs/Cargo.toml inputs/Cargo.lock inputs/rust-toolchain.toml \
      inputs/source/lib.rs inputs/source/main.rs inputs/source/bin/resilience_ensemble.rs \
      inputs/source/bin/robust_set.rs inputs/polar-resilience-scorer.yml \
      inputs/toy-episode.json inputs/toy-ensemble.json inputs/toy-robust-set.json \
      reports/score-report.json reports/ensemble-report.json reports/robust-set-report.json \
      TESTED_COMMIT verify-polar-resilience-receipt.sh > SHA256SUMS
  )
  if verify_bundle "$tmp/escalated-claim" >/dev/null 2>&1; then
    fail "evidence-class escalation unexpectedly passed"
  fi

  printf 'Receipt verifier self-test passed: valid bundle, tampering, omitted checksum, evidence escalation.\n'
  exit 0
fi

if [[ $# -ne 1 ]]; then
  printf 'Usage: %s ARTIFACT_DIRECTORY | --self-test\n' "$0" >&2
  exit 2
fi
verify_bundle "$1"

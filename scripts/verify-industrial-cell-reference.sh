#!/usr/bin/env bash
# Verify the deterministic synthetic industrial-cell reference against a frozen corpus.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
corpus="$repo_root/docs/tech/industrial-cell-fault-corpus-v0.1.json"
expected_corpus_sha256="ea2d0e57d6dc31607b4d70210c85ec21e00ca06c0baf80b74c34885c7b352bf2"

actual_corpus_sha256="$(sha256sum "$corpus" | awk '{print $1}')"
if [[ "$actual_corpus_sha256" != "$expected_corpus_sha256" ]]; then
    echo "FAIL: frozen fault corpus digest mismatch" >&2
    echo "expected=$expected_corpus_sha256" >&2
    echo "actual=$actual_corpus_sha256" >&2
    exit 1
fi
echo "fault_corpus_manifest_sha256=$actual_corpus_sha256"

output="$(mktemp)"
trap 'rm -f "$output"' EXIT

if ! cargo run --locked -p symtropy-bootstrap --example industrial_cell_reference | tee "$output"; then
    echo "FAIL: industrial-cell reference process failed" >&2
    exit 1
fi

require_line() {
    if ! grep -Fqx -- "$1" "$output"; then
        echo "FAIL: expected reference marker missing: $1" >&2
        exit 1
    fi
}

require_line "industrial-cell-reference/v0.2"
require_line "fixture_class=synthetic_not_physical_qualification"
require_line "mass_closure_ppm=850000"
require_line "critical_closure_ppm=800000"
require_line "industrial_closure=BLOCKED"
require_line "valid_process_mass_balance=PASS"
require_line "imbalanced_process_negative_control=REJECTED"
require_line "underfunded_process_negative_control=REJECTED"
require_line "energy_ledger=PASS"
require_line "dominated_candidate=wasteful-import:EXCLUDED"
require_line "fault_scenario=LEDGER-DUPLICATE-EVENT:REJECTED_NO_MUTATION"
require_line "fault_scenario=LEDGER-STALE-SEQUENCE:REJECTED_NO_MUTATION"
require_line "fault_scenario=INVENTORY-OVERDRAW:REJECTED_NO_MUTATION"
require_line "fault_campaign=3/3_SYNTHETIC_NEGATIVE_CONTROLS"
require_line "RESULT=PASS_SYNTHETIC_REFERENCE_CONTRACTS"
require_line "CLAIM_CEILING=no lunar process or robot performance is established"

scenario_count="$(grep -c '^fault_scenario=' "$output" || true)"
if [[ "$scenario_count" != "3" ]]; then
    echo "FAIL: expected exactly 3 frozen fault scenario results; got $scenario_count" >&2
    exit 1
fi

echo "INDEPENDENT_CHECK=PASS_FROZEN_CORPUS_AND_REQUIRED_MARKERS"
echo "CLAIM_CEILING=software_contracts_only_no_physical_qualification"

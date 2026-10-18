#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Run the isolated Q0 tests and emit a schema-shaped, exact-input receipt.
set -u
set -o pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/../.." && pwd)"
CRATE_MANIFEST="$SCRIPT_DIR/Cargo.toml"
TUBE="$SCRIPT_DIR/tube-reference-cases.v0.json"
NETWORK="$SCRIPT_DIR/network-reference-cases.v0.json"
DEFAULT_RECEIPT="$REPO_ROOT/target/organ-transport-q0-receipt.json"
RECEIPT_PATH="${1:-$DEFAULT_RECEIPT}"
SOURCE_COMMIT="${2:-${Q0_SOURCE_COMMIT:-${GITHUB_SHA:-}}}"

if [[ -z "$SOURCE_COMMIT" ]]; then
  SOURCE_COMMIT="$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || true)"
fi
if [[ ! "$SOURCE_COMMIT" =~ ^[[:xdigit:]]{40,64}$ ]]; then
  echo "q0-verifier: source commit must be an explicit 40-64 character Git object ID" >&2
  exit 2
fi
SOURCE_COMMIT="$(printf '%s' "$SOURCE_COMMIT" | tr 'A-F' 'a-f')"

if ! command -v sha256sum >/dev/null 2>&1; then
  echo "q0-verifier: sha256sum is required to bind exact input bytes" >&2
  exit 2
fi

TUBE_SHA="$(sha256sum "$TUBE" | awk '{print $1}')"
NETWORK_SHA="$(sha256sum "$NETWORK" | awk '{print $1}')"
# Domain-separate and length-frame both exact fixture files before hashing the set.
TUBE_BYTES="$(wc -c < "$TUBE" | tr -d ' ')"
NETWORK_BYTES="$(wc -c < "$NETWORK" | tr -d ' ')"
FIXTURE_SHA="$( { printf 'organ-transport-q0-fixtures-v1\0tube:%s\0' "$TUBE_BYTES"; cat "$TUBE"; printf '\0network:%s\0' "$NETWORK_BYTES"; cat "$NETWORK"; } | sha256sum | awk '{print $1}')"
CONFIG_SHA="$(printf '%s\n' \
  'receipt-schema=0.1.0' \
  'tube-model=hagen-poiseuille-rigid-circular-tube-v0' \
  'network-model=prescribed-pressure-hagen-poiseuille-network-v0' \
  'relative-tolerance=1e-12' \
  'absolute-tolerance=1e-18' \
  'edge-flow=(pressure_from-pressure_to)/resistance' \
  'junction-residual=sum(signed-incident-flows)' | sha256sum | awk '{print $1}')"

TOOLCHAIN="unavailable"
if command -v rustc >/dev/null 2>&1; then
  TOOLCHAIN="$(rustc --version)"
fi

FMT_RESULT="not_run"
TEST_RESULT="not_run"
FMT_EXIT=125
TEST_EXIT=125
STATUS="NOT_RUN"
NOTES="Rust/Cargo were unavailable; no Q0 test command was executed."

if command -v cargo >/dev/null 2>&1; then
  if command -v rustfmt >/dev/null 2>&1; then
    if cargo fmt --manifest-path "$CRATE_MANIFEST" -- --check; then
      FMT_RESULT="pass"; FMT_EXIT=0
    else
      FMT_EXIT=$?; FMT_RESULT="fail"
    fi
  else
    FMT_RESULT="blocked"
  fi

  if cargo test --manifest-path "$CRATE_MANIFEST"; then
    TEST_RESULT="pass"; TEST_EXIT=0
  else
    TEST_EXIT=$?; TEST_RESULT="fail"
  fi

  if [[ "$TEST_RESULT" == "fail" || "$FMT_RESULT" == "fail" ]]; then
    STATUS="FAIL"
    NOTES="At least one required Q0 command failed; inspect the preceding command output."
  elif [[ "$TEST_RESULT" == "pass" && "$FMT_RESULT" == "pass" ]]; then
    STATUS="PASS"
    NOTES="PASS is limited to analytic/software conformance on these frozen fixtures; it is not physical or biological validation."
  else
    STATUS="BLOCKED"
    NOTES="The complete Q0 gate did not run because a required tool or formatting check was unavailable."
  fi
fi

mkdir -p "$(dirname -- "$RECEIPT_PATH")"
cat > "$RECEIPT_PATH" <<JSON
{
  "schema_version": "0.1.0",
  "benchmark_id": "ORGAN-TRANSPORT-Q0",
  "context_of_use": {
    "purpose": "Verify the analytical tube reference and prescribed-pressure small-network fixture corpus.",
    "scope": "Analytic/software conformance only; no physical device or tissue inference.",
    "tissue_or_device_profile": null
  },
  "evidence_tier": "analytic_reference",
  "status": "$STATUS",
  "subject": {
    "kind": "analytic_fixture",
    "id": "ORGAN-TRANSPORT-Q0",
    "batch_or_run_id": null
  },
  "software": {
    "code_commit": "$SOURCE_COMMIT",
    "implementation_id": "symtropy-organ-transport-q0",
    "toolchain": "$TOOLCHAIN",
    "solver": "Hagen-Poiseuille analytic edge evaluator and signed junction residual",
    "configuration_sha256": "$CONFIG_SHA"
  },
  "inputs": {
    "fixture_sha256": "$FIXTURE_SHA",
    "units": {
      "pressure": "Pa",
      "length": "m",
      "radius": "m",
      "dynamic_viscosity": "Pa*s",
      "volume_flow": "m^3/s",
      "junction_residual": "m^3/s"
    },
    "boundary_conditions": {
      "node_pressures": "prescribed input at every node; no unknown-pressure network solve",
      "tube_model": "steady laminar Newtonian flow through rigid circular tubes",
      "flow_orientation": "signed flow follows declared edge from-to orientation",
      "tolerances": "relative=1e-12; absolute=1e-18"
    },
    "initial_conditions": null,
    "generator_id": null,
    "generator_seed": null
  },
  "model": {
    "model_id": "hagen-poiseuille-rigid-tube-and-prescribed-pressure-network",
    "version": "0.1.0",
    "assumptions": [
      "steady flow",
      "laminar flow",
      "Newtonian fluid",
      "rigid circular tubes",
      "constant tube radius",
      "all node pressures are prescribed",
      "SI units"
    ],
    "applicability_domain": [
      "analytic equation checks",
      "small synthetic networks with known node pressures",
      "junction mass-conservation checks"
    ],
    "unsupported_conditions": [
      "unknown nodal pressure solving",
      "pulsatile hemodynamics",
      "compliant vessels",
      "endothelial barrier transport",
      "oxygen transport",
      "cell viability",
      "organ function",
      "transplant safety"
    ]
  },
  "metrics": [
    {"metric_id": "format_check_success", "value": $([[ "$FMT_RESULT" == "pass" ]] && echo 1 || echo 0), "unit": "boolean_0_or_1", "reference_value": 1, "absolute_error": null, "relative_error": null, "uncertainty": null, "uncertainty_method": null},
    {"metric_id": "test_command_success", "value": $([[ "$TEST_RESULT" == "pass" ]] && echo 1 || echo 0), "unit": "boolean_0_or_1", "reference_value": 1, "absolute_error": null, "relative_error": null, "uncertainty": null, "uncertainty_method": null},
    {"metric_id": "tube_fixture_count", "value": 5, "unit": "cases", "reference_value": 5, "absolute_error": 0, "relative_error": 0, "uncertainty": null, "uncertainty_method": null},
    {"metric_id": "network_fixture_count", "value": 6, "unit": "cases", "reference_value": 6, "absolute_error": 0, "relative_error": 0, "uncertainty": null, "uncertainty_method": null}
  ],
  "validation": {
    "reference_method": "Hagen-Poiseuille closed-form references, signed edge-flow comparison, and junction mass-balance residuals from the frozen JSON fixtures.",
    "calibration_holdout_separation": "not_applicable",
    "replicates": null,
    "exclusions_and_reasons": [],
    "independent_reviewer": null
  },
  "evidence_sources": [
    {
      "uri": "https://github.com/Luminous-Dynamics/symtropy/blob/$SOURCE_COMMIT/research/organ-transport-q0/tube-reference-cases.v0.json",
      "role": "exact tube fixture corpus",
      "content_sha256": "$TUBE_SHA"
    },
    {
      "uri": "https://github.com/Luminous-Dynamics/symtropy/blob/$SOURCE_COMMIT/research/organ-transport-q0/network-reference-cases.v0.json",
      "role": "exact small-network fixture corpus",
      "content_sha256": "$NETWORK_SHA"
    }
  ],
  "claim_ceiling": [
    "analytic equation/software conformance under explicitly declared assumptions",
    "signed flow comparison and mass balance at declared internal junctions"
  ],
  "notes": "$NOTES FMT_EXIT=$FMT_EXIT; TEST_EXIT=$TEST_EXIT."
}
JSON

echo "Q0 status: $STATUS"
echo "Q0 receipt: $RECEIPT_PATH"
echo "Tube fixture SHA-256: $TUBE_SHA"
echo "Network fixture SHA-256: $NETWORK_SHA"
echo "Combined fixture SHA-256: $FIXTURE_SHA"

case "$STATUS" in
  PASS) exit 0 ;;
  FAIL) exit 1 ;;
  BLOCKED) exit 3 ;;
  *) exit 4 ;;
esac

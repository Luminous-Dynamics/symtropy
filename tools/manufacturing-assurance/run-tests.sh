#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="$ROOT/tools/manufacturing-assurance/composition.rs"
FIXTURES="$ROOT/tools/manufacturing-assurance/fixtures"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

rustc --edition=2021 -D warnings --test "$SRC" -o "$TMP/tests"
"$TMP/tests"

rustc --edition=2021 -D warnings "$SRC" -o "$TMP/verifier"
for mode in integrated unified federated; do
  manifest="$FIXTURES/valid-$mode.tsv"
  first="$("$TMP/verifier" "$manifest")"
  second="$("$TMP/verifier" "$manifest")"
  test "$first" = "$second"
  printf '%s\n' "$first"
done

CAUSAL_SRC="$ROOT/tools/manufacturing-assurance/causal.rs"
rustc --edition=2021 -D warnings --test "$CAUSAL_SRC" -o "$TMP/causal-tests"
"$TMP/causal-tests"

rustc --edition=2021 -D warnings "$CAUSAL_SRC" -o "$TMP/causal-benchmark"
causal_first="$("$TMP/causal-benchmark")"
causal_second="$("$TMP/causal-benchmark")"
test "$causal_first" = "$causal_second"
printf '%s\n' "$causal_first"

CHANGE_SRC="$ROOT/tools/manufacturing-assurance/change_impact.rs"
CHANGE_FIXTURE="$FIXTURES/valid-change-impact.tsv"
rustc --edition=2021 -D warnings --test "$CHANGE_SRC" -o "$TMP/change-tests"
"$TMP/change-tests"

rustc --edition=2021 -D warnings "$CHANGE_SRC" -o "$TMP/change-impact"
change_first="$("$TMP/change-impact" "$CHANGE_FIXTURE")"
change_second="$("$TMP/change-impact" "$CHANGE_FIXTURE")"
test "$change_first" = "$change_second"
printf '%s\n' "$change_first"

DRIFT_SRC="$ROOT/tools/manufacturing-assurance/drift_diagnosis.rs"
rustc --edition=2021 -D warnings --test "$DRIFT_SRC" -o "$TMP/drift-tests"
"$TMP/drift-tests"

rustc --edition=2021 -D warnings "$DRIFT_SRC" -o "$TMP/drift-diagnosis"
drift_first="$("$TMP/drift-diagnosis")"
drift_second="$("$TMP/drift-diagnosis")"
test "$drift_first" = "$drift_second"
printf '%s\n' "$drift_first"

COUPLING_SRC="$ROOT/tools/manufacturing-assurance/composition_coupling.rs"
rustc --edition=2021 -D warnings --test "$COUPLING_SRC" -o "$TMP/coupling-tests"
"$TMP/coupling-tests"

rustc --edition=2021 -D warnings "$COUPLING_SRC" -o "$TMP/composition-coupling"
coupling_first="$("$TMP/composition-coupling")"
coupling_second="$("$TMP/composition-coupling")"
test "$coupling_first" = "$coupling_second"
printf '%s\n' "$coupling_first"

UQ_SRC="$ROOT/tools/manufacturing-assurance/composition_uncertainty.rs"
rustc --edition=2021 -D warnings --test "$UQ_SRC" -o "$TMP/uq-tests"
"$TMP/uq-tests"

rustc --edition=2021 -D warnings "$UQ_SRC" -o "$TMP/composition-uncertainty"
uq_first="$("$TMP/composition-uncertainty")"
uq_second="$("$TMP/composition-uncertainty")"
test "$uq_first" = "$uq_second"
printf '%s\\n' "$uq_first"

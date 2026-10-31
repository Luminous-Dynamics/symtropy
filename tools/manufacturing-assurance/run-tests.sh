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

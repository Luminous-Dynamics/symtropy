#!/usr/bin/env bash
set -euo pipefail

matrix="fixtures/av-observation-001/spatial-claim-calibration-matrix-v0.tsv"
[[ -f "$matrix" ]] || { echo "missing matrix: $matrix" >&2; exit 1; }

awk -F '|' '
  BEGIN { data_rows = 0; bad = 0 }
  /^#/ || NF == 0 { next }
  {
    data_rows++
    if (NF != 10) {
      printf "line %d: expected 10 fields, got %d\n", NR, NF > "/dev/stderr"
      bad = 1
      next
    }
    for (i = 1; i <= 10; i++) {
      if ($i == "") {
        printf "line %d: empty field %d\n", NR, i > "/dev/stderr"
        bad = 1
      }
    }
    if (++case_id[$1] != 1) {
      printf "line %d: duplicate case_id %s\n", NR, $1 > "/dev/stderr"
      bad = 1
    }
    pair_count[$3]++
    variant_count[$3 SUBSEP $4]++
    if ($4 != "control" && $4 != "mutated") {
      printf "line %d: invalid variant %s\n", NR, $4 > "/dev/stderr"
      bad = 1
    }
    # Field 7 is candidate-visible metadata. Hidden truth/witness labels
    # belong in field 8 only and must never be injected into this field.
    metadata = tolower($7)
    if (metadata ~ /(evaluator_truth|hidden_world_id|truth_id|canonical_witness_id|future_frame_payload)/) {
      printf "line %d: evaluator-only token in candidate metadata\n", NR > "/dev/stderr"
      bad = 1
    }
  }
  END {
    if (data_rows != 16) {
      printf "expected 16 data rows, got %d\n", data_rows > "/dev/stderr"
      bad = 1
    }
    pairs = 0
    for (pair in pair_count) {
      pairs++
      if (pair_count[pair] != 2 ||
          variant_count[pair SUBSEP "control"] != 1 ||
          variant_count[pair SUBSEP "mutated"] != 1) {
        printf "pair %s must contain exactly one control and one mutated row\n", pair > "/dev/stderr"
        bad = 1
      }
    }
    if (pairs != 8) {
      printf "expected 8 paired groups, got %d\n", pairs > "/dev/stderr"
      bad = 1
    }
    if (bad) exit 1
    printf "PASS: %d unique cases, %d complete control/mutated pairs, 10 fields per row\n", data_rows, pairs
  }
' "$matrix"

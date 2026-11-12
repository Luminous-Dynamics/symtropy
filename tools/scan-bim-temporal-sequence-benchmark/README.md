# Scan-to-BIM Temporal Sequence Benchmark v0.1

A bounded Rust evaluator for synthetic, multi-acquisition building sequences. It evaluates persistent component association, identity switches, track fragmentation, state-event attribution and the evidence that each decision claims to have consumed.

## What it evaluates

- **IDF1:** global one-to-one identity assignment over visible evaluator observations. The implementation uses a bounded exact maximum-weight bipartite assignment (at most 16 ground-truth entities and 16 candidate tracks per sequence). IDTP is the maximum assigned overlap; IDFP = candidate detections - IDTP; IDFN = visible reference detections - IDTP.
- **Identity switches:** changes of candidate track identity for the same persistent evaluator entity across successive visible acquisitions.
- **Fragmentation:** a visible entity is detected, then missed while still visible, then detected again.
- **Event attribution:** unchanged, viewpoint-only, occlusion, registration drift, physical change, replacement and unresolved state events are compared with the candidate's reported event and disposition.
- **False physical-change events:** a nuisance/unresolved reference event reported as physical change or admitted as a physical delta.
- **Temporal evidence violations:** candidate evidence references that are missing, duplicated, first received after cutoff, measured after cutoff, outside their validity interval, revoked or superseded by the cutoff, unsupported for the decision purpose, outside the declared clock domain, or too coarse for the decision's declared timestamp resolution.
- **Replacement semantics:** physical replacement has a new evaluator entity ID; it is not scored as an identity switch of the retired component.

## Candidate/reference separation

The CLI takes two separate files:

- `fixtures/candidate-sequences.json`: candidate-facing outputs, with candidate track IDs, reported events, dispositions and evidence event references.
- `fixtures/evaluator-reference.json`: evaluator-only building groups, ground-truth identities/events/visibility and evidence event timing/validity/revocation facts.

The candidate must never be given the evaluator-reference file, hidden identities, event labels or building holdout assignments. This P0 CLI reads both files in one scoring process only after candidate outputs exist. It is not a sandbox or a proof of organizational separation.

## Temporal contract

All v0.1 clocks use the explicitly declared synthetic clock `synthetic-global-tick-v0`. Unknown/unmapped clock domains are violations. Both measurement/event time and first-received/knowledge time must be no later than the decision cutoff. The evidence validity interval is half-open: `[valid_from_tick, valid_until_tick)`; null `valid_until_tick` means no declared end. A revocation known at or before the cutoff excludes the evidence. A revocation or supersession first known after the cutoff does not rewrite the historical snapshot. A supersession must bind an exact successor event whose first-received time is no later than the declared supersession-known time.

The evaluator checks temporal eligibility, it does not implement general clock conversion, evidence selection, signature verification or the owning bitemporal snapshot engine. Those remain with #4705/#4749 and source authority owners. It verifies only the synthetic event references supplied to it.

## Run

```sh
cargo fmt --manifest-path tools/scan-bim-temporal-sequence-benchmark/Cargo.toml -- --check
cargo test --locked --manifest-path tools/scan-bim-temporal-sequence-benchmark/Cargo.toml
cargo clippy --locked --manifest-path tools/scan-bim-temporal-sequence-benchmark/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path tools/scan-bim-temporal-sequence-benchmark/Cargo.toml -- \
  --candidate tools/scan-bim-temporal-sequence-benchmark/fixtures/candidate-sequences.json \
  --reference tools/scan-bim-temporal-sequence-benchmark/fixtures/evaluator-reference.json
```

The frozen corpus contains seven synthetic sequences and seven synthetic building groups. The Rust crate currently has 19 unit tests covering known-answer scoring, candidate/reference ordering, ID switches, spurious detections, missed visible detections, occlusion/replacement semantics, nuisance false-change events, late evidence, NDT label leakage, clock identity/precision, expired calibration, revocation and supersession timing, and duplicate event identity. These are known-answer fixtures, not an external generalization sample.

## Claim ceiling

P0 checks evaluator semantics on synthetic data only. It does not prove real physical component identity, sensor or survey accuracy, registration stability, building safety, code compliance, or SOTA superiority. External comparison requires the CV4AEC 2024 data with exact rights, timestamped multi-acquisition sequences, independent building-level references and matched baselines.
